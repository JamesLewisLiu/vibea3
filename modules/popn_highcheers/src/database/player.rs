use bson::{DateTime, doc};
use vibea3::{FindOneAndUpdateOptions, FindOptions, ReturnDocument, UpdateOptions};

use super::{
    CHARACTER_USAGE, COUNTERS, COURSES, Database, ERRORS, MUSIC_USAGE, PLAYERS, SCORES, decode,
    encode,
    model::{CourseRecord, MusicOption, MusicScore, PlayerProfile, ProfilePatch},
};
use crate::protocol::PLAYABLE_SHEET_COUNT;

impl Database {
    pub(crate) async fn profile(
        &self,
        ref_id: &str,
        data_id: &str,
    ) -> Result<Option<PlayerProfile>, String> {
        let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? else {
            return Ok(None);
        };
        self.raw
            .find_one(PLAYERS.into(), doc! { "_id": user_id }, Default::default())
            .await?
            .map(decode::<PlayerProfile>)
            .transpose()
    }

    pub(crate) async fn create_profile(
        &self,
        ref_id: &str,
        data_id: &str,
        name: String,
        pref: i8,
    ) -> Result<Option<PlayerProfile>, String> {
        let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? else {
            return Ok(None);
        };
        if let Some(profile) = self
            .raw
            .find_one(PLAYERS.into(), doc! { "_id": &user_id }, Default::default())
            .await?
            .map(decode::<PlayerProfile>)
            .transpose()?
        {
            self.bind_game_profile(&profile.user_id).await?;
            return Ok(Some(profile));
        }
        let g_pm_id = format!("{:012}", self.next_counter("g_pm_id").await?);
        let profile = PlayerProfile::new(user_id, g_pm_id, name, pref);
        self.raw
            .insert_one(PLAYERS.into(), encode(&profile)?)
            .await?;
        self.bind_game_profile(&profile.user_id).await?;
        Ok(Some(profile))
    }

    pub(crate) async fn patch_profile(
        &self,
        ref_id: &str,
        data_id: &str,
        patch: ProfilePatch,
    ) -> Result<(), String> {
        let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? else {
            return Ok(());
        };
        let mut set = doc! { "updated_at": DateTime::now() };
        set_optional(&mut set, "tutorial", patch.tutorial)?;
        set_optional(&mut set, "read_news", patch.read_news)?;
        set_optional(&mut set, "latest_music", patch.latest_music)?;
        set_optional(&mut set, "nice", patch.nice)?;
        set_optional(&mut set, "favorite_chara", patch.favorite_chara)?;
        set_optional(&mut set, "popn_class", patch.popn_class)?;
        set_optional(&mut set, "power_point", patch.power_point)?;
        set_optional(&mut set, "power_point_list", patch.power_point_list)?;
        set_optional(&mut set, "sc_news_no", patch.sc_news_no)?;
        set_optional(&mut set, "read_policy", patch.read_policy)?;
        set_optional(&mut set, "language", patch.language)?;
        set_optional(&mut set, "ep", patch.ep)?;
        set_optional(&mut set, "estatus", patch.estatus)?;
        set_optional(&mut set, "customize", patch.customize)?;
        set_optional(&mut set, "option", patch.option)?;
        set_optional(&mut set, "config", patch.config)?;
        set_optional(&mut set, "items", patch.items)?;
        set_optional(&mut set, "characters", patch.characters)?;
        set_optional(&mut set, "extra", patch.extra)?;
        set_optional(&mut set, "netvs", patch.netvs)?;
        set_optional(&mut set, "event", patch.event)?;
        self.raw
            .update_one(
                PLAYERS.into(),
                doc! { "_id": user_id },
                doc! { "$set": set }.into(),
                Default::default(),
            )
            .await
    }

    pub(crate) async fn scores(
        &self,
        ref_id: &str,
        data_id: &str,
    ) -> Result<Vec<MusicScore>, String> {
        let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? else {
            return Ok(Vec::new());
        };
        self.raw
            .find_many(
                SCORES.into(),
                doc! { "user_id": user_id },
                FindOptions {
                    sort: Some(doc! { "music_num": 1, "sheet_num": 1 }),
                    limit: None,
                },
            )
            .await?
            .into_iter()
            .map(decode)
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn save_score(
        &self,
        ref_id: &str,
        data_id: &str,
        music_num: i16,
        sheet_num: u8,
        score: i32,
        clear_type: u8,
        clear_rank: u8,
        chara_num: i16,
        option: Option<MusicOption>,
    ) -> Result<(), String> {
        if music_num.is_negative() || usize::from(sheet_num) >= PLAYABLE_SHEET_COUNT {
            return Ok(());
        }
        let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? else {
            return Ok(());
        };
        let id = format!("{user_id}:{music_num}:{sheet_num}");
        let existing = self
            .raw
            .find_one(SCORES.into(), doc! { "_id": &id }, Default::default())
            .await?
            .map(decode::<MusicScore>)
            .transpose()?;
        let better = existing.as_ref().is_none_or(|old| score > old.score);
        let mut value = existing.unwrap_or(MusicScore {
            id: id.clone(),
            user_id,
            music_num,
            sheet_num,
            score: 0,
            clear_type: 0,
            clear_rank: 0,
            cnt: 0,
            score_ver: 0,
            clear_type_ver: 0,
            option: None,
            updated_at: DateTime::now(),
        });
        value.cnt = value.cnt.saturating_add(1);
        if better {
            value.score = score;
            value.clear_type = clear_type;
            value.clear_rank = clear_rank;
            value.score_ver = score;
            value.clear_type_ver = clear_type;
        } else {
            value.clear_type = value.clear_type.max(clear_type);
            value.clear_rank = value.clear_rank.max(clear_rank);
        }
        if option.is_some() {
            value.option = option;
        }
        value.updated_at = DateTime::now();
        let mut document = encode(&value)?;
        document.remove("_id");
        self.raw
            .update_one(
                SCORES.into(),
                doc! { "_id": id },
                doc! { "$set": document }.into(),
                UpdateOptions { upsert: true },
            )
            .await?;
        self.increment_usage(MUSIC_USAGE, i32::from(music_num))
            .await?;
        if chara_num >= 0 {
            self.increment_usage(CHARACTER_USAGE, i32::from(chara_num))
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn music_option(
        &self,
        ref_id: &str,
        data_id: &str,
        music_num: i16,
        sheet_num: u8,
    ) -> Result<Option<MusicOption>, String> {
        let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? else {
            return Ok(None);
        };
        self.raw
            .find_one(
                SCORES.into(),
                doc! { "_id": format!("{user_id}:{music_num}:{sheet_num}") },
                Default::default(),
            )
            .await?
            .map(decode::<MusicScore>)
            .transpose()
            .map(|score| score.and_then(|score| score.option))
    }

    pub(crate) async fn set_lumina(&self, ref_id: &str, lumina: i32) -> Result<(), String> {
        let Some(user_id) = self.user_id(ref_id, None).await? else {
            return Ok(());
        };
        self.raw
            .update_one(
                PLAYERS.into(),
                doc! { "_id": user_id },
                doc! { "$set": { "lumina": lumina, "updated_at": DateTime::now() } }.into(),
                Default::default(),
            )
            .await
    }

    pub(crate) async fn save_course(
        &self,
        ref_id: &str,
        data_id: &str,
        mut course: CourseRecord,
    ) -> Result<(), String> {
        let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? else {
            return Ok(());
        };
        course.user_id.clone_from(&user_id);
        course.id = format!("{user_id}:{}", course.course_id);
        course.updated_at = DateTime::now();
        let mut document = encode(&course)?;
        document.remove("_id");
        self.raw
            .update_one(
                COURSES.into(),
                doc! { "_id": course.id },
                doc! { "$set": document }.into(),
                UpdateOptions { upsert: true },
            )
            .await
    }

    pub(crate) async fn delete_profile(&self, ref_id: &str) -> Result<(), String> {
        let Some(user_id) = self.user_id(ref_id, None).await? else {
            return Ok(());
        };
        self.raw
            .delete_one(PLAYERS.into(), doc! { "_id": &user_id })
            .await?;
        self.raw
            .delete_many(SCORES.into(), doc! { "user_id": &user_id })
            .await?;
        self.raw
            .delete_many(COURSES.into(), doc! { "user_id": user_id })
            .await
    }

    pub(crate) async fn record_error(&self, document: bson::Document) -> Result<(), String> {
        self.raw.insert_one(ERRORS.into(), document).await
    }

    pub(crate) async fn save_cabinet_name(&self, srcid: &str, name: String) -> Result<(), String> {
        self.raw
            .update_one(
                super::CABINETS.into(),
                doc! { "_id": srcid },
                doc! { "$set": { "name": name, "updated_at": DateTime::now() } }.into(),
                UpdateOptions { upsert: true },
            )
            .await
    }

    pub(crate) async fn next_play_id(&self) -> Result<i32, String> {
        Ok(self.next_counter("play_id").await?.min(i32::MAX as i64) as i32)
    }

    pub(crate) async fn next_lobby_id(&self) -> Result<u32, String> {
        Ok(self.next_counter("lobby_room").await?.min(u32::MAX as i64) as u32)
    }

    pub(crate) async fn begin_play(&self, ref_id: &str, data_id: &str) -> Result<i32, String> {
        if let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? {
            self.raw
                .update_one(
                    PLAYERS.into(),
                    doc! { "_id": user_id },
                    doc! {
                        "$inc": { "total_play_cnt": 1 },
                        "$set": { "updated_at": DateTime::now() },
                    }
                    .into(),
                    Default::default(),
                )
                .await?;
        }
        self.next_play_id().await
    }

    async fn next_counter(&self, name: &str) -> Result<i64, String> {
        let document = self
            .raw
            .find_one_and_update(
                COUNTERS.into(),
                doc! { "_id": name },
                doc! { "$inc": { "value": 1_i64 } }.into(),
                FindOneAndUpdateOptions {
                    upsert: true,
                    return_document: ReturnDocument::After,
                },
            )
            .await?
            .ok_or_else(|| format!("counter {name} did not return a value"))?;
        document
            .get_i64("value")
            .or_else(|_| document.get_i32("value").map(i64::from))
            .map_err(|error| error.to_string())
    }
}

fn set_optional<T: serde::Serialize>(
    document: &mut bson::Document,
    key: &str,
    value: Option<T>,
) -> Result<(), String> {
    if let Some(value) = value {
        document.insert(
            key,
            bson::to_bson(&value).map_err(|error| error.to_string())?,
        );
    }
    Ok(())
}
