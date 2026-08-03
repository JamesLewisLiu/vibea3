mod model;

use std::sync::Arc;

use bson::{Bson, DateTime, Document, doc};
use serde::{Serialize, de::DeserializeOwned};
use vibea3::{
    DocumentDatabase, FindOneAndUpdateOptions, FindOptions, IndexDefinition, ReturnDocument,
    UpdateOptions,
};

use crate::protocol::GAME_MODEL;
pub(crate) use model::{
    MatchingResult, MusicScore, PlayRecord, PlayerItem, PlayerParam, PlayerProfile, PlayerSetting,
    RadarElements, StoryProgress,
};

const SESSIONS: &str = "card_sessions";
const GAME_BINDINGS: &str = "game_bindings";
const PLAYERS: &str = "sdvx7_players";
const SCORES: &str = "sdvx7_scores";
const PLAYS: &str = "sdvx7_plays";
const CABINETS: &str = "sdvx7_cabinets";
const AUTOMATIONS: &str = "sdvx7_automations";
const COUNTERS: &str = "sdvx7_counters";

#[derive(Clone)]
pub(crate) struct Database {
    raw: Arc<dyn DocumentDatabase>,
}

impl Database {
    pub(crate) fn new(raw: Arc<dyn DocumentDatabase>) -> Self {
        Self { raw }
    }

    pub(crate) async fn initialize(&self) -> Result<(), String> {
        self.raw
            .create_indexes(
                PLAYERS.into(),
                vec![IndexDefinition {
                    keys: doc! { "code": 1 },
                    name: Some("code_unique".into()),
                    unique: true,
                    expire_after_seconds: None,
                }],
            )
            .await?;
        self.raw
            .create_indexes(
                SCORES.into(),
                vec![
                    IndexDefinition {
                        keys: doc! { "user_id": 1, "music_id": 1, "music_type": 1 },
                        name: Some("user_chart_unique".into()),
                        unique: true,
                        expire_after_seconds: None,
                    },
                    IndexDefinition {
                        keys: doc! { "music_id": 1, "music_type": 1, "score": -1 },
                        name: Some("chart_hiscore".into()),
                        unique: false,
                        expire_after_seconds: None,
                    },
                ],
            )
            .await?;
        self.raw
            .create_indexes(
                PLAYS.into(),
                vec![IndexDefinition {
                    keys: doc! { "user_id": 1, "play_id": -1 },
                    name: Some("user_play_history".into()),
                    unique: false,
                    expire_after_seconds: None,
                }],
            )
            .await?;
        self.backfill_bindings().await
    }

    pub(crate) async fn user_id(
        &self,
        ref_id: &str,
        claimed: Option<&str>,
    ) -> Result<Option<String>, String> {
        let session = self
            .raw
            .find_one(
                SESSIONS.into(),
                doc! { "_id": ref_id, "expires_at": { "$gt": DateTime::now() } },
                Default::default(),
            )
            .await?;
        Ok(session
            .and_then(|document| document.get_str("user_id").ok().map(str::to_owned))
            .filter(|user_id| claimed.is_none_or(|claimed| claimed == user_id)))
    }

    pub(crate) async fn create_profile(&self, profile: &PlayerProfile) -> Result<(), String> {
        self.raw
            .update_one(
                PLAYERS.into(),
                doc! { "_id": &profile.user_id },
                doc! { "$setOnInsert": encode(profile)? }.into(),
                UpdateOptions { upsert: true },
            )
            .await?;
        self.bind_profile(&profile.user_id).await
    }

    pub(crate) async fn profile(&self, user_id: &str) -> Result<Option<PlayerProfile>, String> {
        self.raw
            .find_one(PLAYERS.into(), doc! { "_id": user_id }, Default::default())
            .await?
            .map(decode)
            .transpose()
    }

    pub(crate) async fn save_profile(&self, profile: &mut PlayerProfile) -> Result<(), String> {
        profile.updated_at = DateTime::now();
        let mut fields = encode(profile)?;
        fields.remove("_id");
        self.raw
            .update_one(
                PLAYERS.into(),
                doc! { "_id": &profile.user_id },
                doc! { "$set": fields }.into(),
                UpdateOptions { upsert: false },
            )
            .await
    }

    pub(crate) async fn scores(&self, user_id: &str) -> Result<Vec<MusicScore>, String> {
        self.raw
            .find_many(
                SCORES.into(),
                doc! { "user_id": user_id },
                FindOptions {
                    sort: Some(doc! { "music_id": 1, "music_type": 1 }),
                    limit: None,
                },
            )
            .await?
            .into_iter()
            .map(decode)
            .collect()
    }

    pub(crate) async fn upsert_score(&self, incoming: &MusicScore) -> Result<(), String> {
        let current = self
            .raw
            .find_one(
                SCORES.into(),
                doc! { "_id": &incoming.id },
                Default::default(),
            )
            .await?
            .map(decode::<MusicScore>)
            .transpose()?;
        let mut score = incoming.clone();
        if let Some(old) = current {
            score.score = score.score.max(old.score);
            score.exscore = score.exscore.max(old.exscore);
            score.clear_type = score.clear_type.max(old.clear_type);
            score.score_grade = score.score_grade.max(old.score_grade);
            score.max_chain = score.max_chain.max(old.max_chain);
            score.best_critical = score.best_critical.max(old.best_critical);
            score.best_near = if old.play_count == 0 {
                score.best_near
            } else {
                score.best_near.min(old.best_near)
            };
            score.best_error = if old.play_count == 0 {
                score.best_error
            } else {
                score.best_error.min(old.best_error)
            };
            score.volforce = score.volforce.max(old.volforce);
            score.just = score.just.max(old.just);
            score.effective_rate = score.effective_rate.max(old.effective_rate);
            score.btn_rate = score.btn_rate.max(old.btn_rate);
            score.long_rate = score.long_rate.max(old.long_rate);
            score.vol_rate = score.vol_rate.max(old.vol_rate);
            score.play_count = old.play_count.saturating_add(1);
            score.clear_count = old.clear_count.saturating_add(incoming.clear_count);
            score.ultimate_chain_count = old
                .ultimate_chain_count
                .saturating_add(incoming.ultimate_chain_count);
            score.perfect_ultimate_chain_count = old
                .perfect_ultimate_chain_count
                .saturating_add(incoming.perfect_ultimate_chain_count);
        }
        self.raw
            .update_one(
                SCORES.into(),
                doc! { "_id": &score.id },
                doc! { "$set": encode(&score)? }.into(),
                UpdateOptions { upsert: true },
            )
            .await
    }

    pub(crate) async fn hiscores(&self, limit: u32, offset: u32) -> Result<Vec<Document>, String> {
        let limit = u64::from(limit.saturating_add(offset));
        let mut rows = self
            .raw
            .find_many(
                SCORES.into(),
                doc! {},
                FindOptions {
                    sort: Some(doc! { "score": -1, "updated_at": 1 }),
                    limit: Some(limit),
                },
            )
            .await?;
        rows.drain(..rows.len().min(offset as usize));
        Ok(rows)
    }

    pub(crate) async fn next_play_id(&self) -> Result<i64, String> {
        let result = self
            .raw
            .find_one_and_update(
                COUNTERS.into(),
                doc! { "_id": "play_id" },
                doc! { "$inc": { "value": 1_i64 } }.into(),
                FindOneAndUpdateOptions {
                    upsert: true,
                    return_document: ReturnDocument::After,
                },
            )
            .await?
            .ok_or_else(|| "play counter update returned no document".to_owned())?;
        match result.get("value") {
            Some(Bson::Int64(value)) => Ok(*value),
            Some(Bson::Int32(value)) => Ok(i64::from(*value)),
            _ => Err("play counter has invalid value".into()),
        }
    }

    pub(crate) async fn start_play(&self, record: &PlayRecord) -> Result<(), String> {
        self.raw.insert_one(PLAYS.into(), encode(record)?).await
    }

    pub(crate) async fn end_play(&self, play_id: i64) -> Result<(), String> {
        self.raw
            .update_one(
                PLAYS.into(),
                doc! { "_id": play_id },
                doc! { "$set": { "ended_at": DateTime::now() } }.into(),
                UpdateOptions::default(),
            )
            .await
    }

    pub(crate) async fn save_cabinet(
        &self,
        location_id: &str,
        document: Document,
    ) -> Result<(), String> {
        self.raw
            .update_one(
                CABINETS.into(),
                doc! { "_id": location_id },
                doc! { "$set": document }.into(),
                UpdateOptions { upsert: true },
            )
            .await
    }

    pub(crate) async fn save_automation(
        &self,
        user_id: &str,
        document: Document,
    ) -> Result<(), String> {
        self.raw
            .update_one(
                AUTOMATIONS.into(),
                doc! { "_id": user_id },
                doc! { "$set": document }.into(),
                UpdateOptions { upsert: true },
            )
            .await
    }

    async fn bind_profile(&self, user_id: &str) -> Result<(), String> {
        let now = DateTime::now();
        self.raw.update_one(
            GAME_BINDINGS.into(), doc! { "_id": format!("{GAME_MODEL}:{user_id}") },
            doc! { "$set": { "model": GAME_MODEL, "user_id": user_id, "updated_at": now }, "$setOnInsert": { "created_at": now } }.into(),
            UpdateOptions { upsert: true },
        ).await
    }

    async fn backfill_bindings(&self) -> Result<(), String> {
        for document in self
            .raw
            .find_many(PLAYERS.into(), doc! {}, Default::default())
            .await?
        {
            if let Ok(user_id) = document.get_str("_id") {
                self.bind_profile(user_id).await?;
            }
        }
        Ok(())
    }
}

fn encode<T: Serialize>(value: &T) -> Result<Document, String> {
    bson::to_document(value).map_err(|error| error.to_string())
}

fn decode<T: DeserializeOwned>(document: Document) -> Result<T, String> {
    bson::from_document(document).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Mutex};

    use vibea3::{DbFuture, FindOneOptions, Update};

    use super::*;

    #[derive(Default)]
    struct MemoryDatabase {
        collections: Mutex<HashMap<String, Vec<Document>>>,
    }

    impl DocumentDatabase for MemoryDatabase {
        fn create_indexes(
            &self,
            _collection: String,
            _indexes: Vec<IndexDefinition>,
        ) -> DbFuture<()> {
            Box::pin(async { Ok(()) })
        }

        fn find_one(
            &self,
            collection: String,
            filter: Document,
            _options: FindOneOptions,
        ) -> DbFuture<Option<Document>> {
            let result = self
                .collections
                .lock()
                .unwrap()
                .get(&collection)
                .and_then(|documents| {
                    documents
                        .iter()
                        .find(|document| matches_filter(document, &filter))
                        .cloned()
                });
            Box::pin(async move { Ok(result) })
        }

        fn find_many(
            &self,
            collection: String,
            filter: Document,
            _options: FindOptions,
        ) -> DbFuture<Vec<Document>> {
            let result = self
                .collections
                .lock()
                .unwrap()
                .get(&collection)
                .into_iter()
                .flatten()
                .filter(|document| matches_filter(document, &filter))
                .cloned()
                .collect();
            Box::pin(async move { Ok(result) })
        }

        fn update_one(
            &self,
            collection: String,
            filter: Document,
            update: Update,
            options: UpdateOptions,
        ) -> DbFuture<()> {
            let mut collections = self.collections.lock().unwrap();
            let documents = collections.entry(collection).or_default();
            let existing = documents
                .iter_mut()
                .find(|document| matches_filter(document, &filter));
            let update = match update {
                Update::Document(document) => document,
                Update::Pipeline(_) => {
                    return Box::pin(async { Err("pipeline unsupported".into()) });
                }
            };
            if let Some(document) = existing {
                apply_fields(document, update.get_document("$set").ok());
            } else if options.upsert {
                let mut document = filter;
                apply_fields(&mut document, update.get_document("$setOnInsert").ok());
                apply_fields(&mut document, update.get_document("$set").ok());
                documents.push(document);
            }
            Box::pin(async { Ok(()) })
        }

        fn find_one_and_update(
            &self,
            _collection: String,
            _filter: Document,
            _update: Update,
            _options: FindOneAndUpdateOptions,
        ) -> DbFuture<Option<Document>> {
            Box::pin(async { Err("find_one_and_update unsupported".into()) })
        }

        fn insert_one(&self, collection: String, document: Document) -> DbFuture<()> {
            self.collections
                .lock()
                .unwrap()
                .entry(collection)
                .or_default()
                .push(document);
            Box::pin(async { Ok(()) })
        }

        fn insert_many(&self, collection: String, documents: Vec<Document>) -> DbFuture<()> {
            self.collections
                .lock()
                .unwrap()
                .entry(collection)
                .or_default()
                .extend(documents);
            Box::pin(async { Ok(()) })
        }
    }

    fn matches_filter(document: &Document, filter: &Document) -> bool {
        filter
            .iter()
            .all(|(name, value)| document.get(name) == Some(value))
    }

    fn apply_fields(document: &mut Document, fields: Option<&Document>) {
        if let Some(fields) = fields {
            for (name, value) in fields {
                document.insert(name, value.clone());
            }
        }
    }

    #[tokio::test]
    async fn profile_registration_is_idempotent_and_keyed_by_global_user_id() {
        let database = Database::new(Arc::new(MemoryDatabase::default()));
        let first = PlayerProfile::new("4164070139934008".into(), "FIRST".into());
        database.create_profile(&first).await.unwrap();
        let duplicate = PlayerProfile::new(first.user_id.clone(), "SECOND".into());
        database.create_profile(&duplicate).await.unwrap();

        let loaded = database.profile(&first.user_id).await.unwrap().unwrap();
        assert_eq!(loaded.user_id, first.user_id);
        assert_eq!(loaded.name, "FIRST");
    }
}
