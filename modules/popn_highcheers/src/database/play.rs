use bson::{DateTime, doc};
use serde::{Deserialize, Serialize};
use vibea3::UpdateOptions;

use super::{Database, PLAYS, encode};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlayStage {
    pub music_num: i16,
    pub sheet_num: u8,
    pub clear_rank: u8,
    pub clear_type: u8,
    pub score: i32,
    pub cool: i16,
    pub great: i16,
    pub good: i16,
    pub bad: i16,
    pub combo: i16,
    pub highlight: i16,
    pub gauge: i16,
    pub gauge_type: i8,
    pub is_win: i8,
    pub matching: i8,
    pub ojama_vs: i8,
}

#[derive(Serialize)]
struct PlayRecord {
    #[serde(rename = "_id")]
    id: String,
    user_id: String,
    play_id: i32,
    start_type: Option<i8>,
    shop_name: String,
    pref: i8,
    stages: Vec<PlayStage>,
    updated_at: DateTime,
}

impl Database {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn save_play(
        &self,
        ref_id: &str,
        data_id: &str,
        play_id: i32,
        start_type: Option<i8>,
        shop_name: String,
        pref: i8,
        stages: Vec<PlayStage>,
    ) -> Result<(), String> {
        let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? else {
            return Ok(());
        };
        let id = format!("{user_id}:{play_id}");
        let mut record = encode(&PlayRecord {
            id: id.clone(),
            user_id,
            play_id,
            start_type,
            shop_name,
            pref,
            stages,
            updated_at: DateTime::now(),
        })?;
        record.remove("_id");
        self.raw
            .update_one(
                PLAYS.into(),
                doc! { "_id": id },
                doc! { "$set": record }.into(),
                UpdateOptions { upsert: true },
            )
            .await
    }
}
