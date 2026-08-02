use std::collections::HashSet;

use bson::{DateTime, doc};
use vibea3::{FindOptions, UpdateOptions};

use super::{
    CHARACTER_USAGE, Database, MUSIC_USAGE, SCORES, decode,
    model::{UsageCounter, UsageSnapshot},
};
use crate::protocol::{POPULAR_CHARACTER_CAPACITY, POPULAR_MUSIC_CAPACITY, RECOMMENDATION_COUNT};

const NO_RECOMMENDATION: i32 = -1;

impl Database {
    pub(crate) async fn usage_snapshot(
        &self,
        identity: Option<(&str, &str)>,
    ) -> Result<UsageSnapshot, String> {
        let music = self.top_usage(MUSIC_USAGE, POPULAR_MUSIC_CAPACITY).await?;
        let characters = self
            .top_usage(CHARACTER_USAGE, POPULAR_CHARACTER_CAPACITY)
            .await?;
        let played = if let Some((ref_id, data_id)) = identity {
            self.played_music(ref_id, data_id).await?
        } else {
            HashSet::new()
        };

        let popular_music: Vec<_> = music
            .iter()
            .filter_map(|entry| i16::try_from(entry.id).ok())
            .filter(|music_num| !music_num.is_negative())
            .collect();
        let popular_characters = characters
            .into_iter()
            .filter_map(|entry| i16::try_from(entry.id).ok())
            .filter(|chara_num| *chara_num >= 0)
            .collect();
        let mut recommend_music: Vec<_> = popular_music
            .iter()
            .copied()
            .filter(|music_num| !played.contains(music_num))
            .take(RECOMMENDATION_COUNT)
            .map(i32::from)
            .collect();
        if !recommend_music.is_empty() {
            recommend_music.resize(RECOMMENDATION_COUNT, NO_RECOMMENDATION);
        }

        Ok(UsageSnapshot {
            popular_characters,
            popular_music,
            recommend_music,
        })
    }

    pub(super) async fn increment_usage(&self, collection: &str, id: i32) -> Result<(), String> {
        self.raw
            .update_one(
                collection.into(),
                doc! { "_id": id },
                doc! {
                    "$inc": { "plays": 1_i64 },
                    "$set": { "updated_at": DateTime::now() },
                }
                .into(),
                UpdateOptions { upsert: true },
            )
            .await
    }

    async fn top_usage(&self, collection: &str, limit: u64) -> Result<Vec<UsageCounter>, String> {
        self.raw
            .find_many(
                collection.into(),
                doc! {},
                FindOptions {
                    sort: Some(doc! { "plays": -1, "updated_at": -1, "_id": 1 }),
                    limit: Some(limit),
                },
            )
            .await?
            .into_iter()
            .map(decode)
            .collect()
    }

    async fn played_music(&self, ref_id: &str, data_id: &str) -> Result<HashSet<i16>, String> {
        let Some(user_id) = self.user_id(ref_id, Some(data_id)).await? else {
            return Ok(HashSet::new());
        };
        Ok(self
            .raw
            .find_many(
                SCORES.into(),
                doc! { "user_id": user_id },
                FindOptions {
                    sort: None,
                    limit: None,
                },
            )
            .await?
            .into_iter()
            .filter_map(|document| document.get_i32("music_num").ok())
            .filter_map(|music_num| i16::try_from(music_num).ok())
            .collect())
    }
}
