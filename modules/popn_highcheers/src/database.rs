mod lobby;
mod model;
mod play;
mod player;
mod usage;

use std::sync::Arc;

use bson::{DateTime, Document, doc};
use serde::{Deserialize, Serialize};
use vibea3::{DocumentDatabase, IndexDefinition};

pub(crate) use lobby::{LobbyEntry, LobbyRoom};
pub(crate) use model::{
    CharacterState, ConfigState, CourseRecord, EventBasket, EventState, ExtraState, ItemState,
    MusicOption, MusicScore, NetvsState, PlayerProfile, ProfilePatch, UsageSnapshot,
};
pub(crate) use play::PlayStage;

const SESSIONS: &str = "card_sessions";
const GAME_BINDINGS: &str = "game_bindings";
const PLAYERS: &str = "popn29_players";
const SCORES: &str = "popn29_scores";
const COURSES: &str = "popn29_courses";
const PLAYS: &str = "popn29_plays";
const ROOMS: &str = "popn29_rooms";
const CABINETS: &str = "popn29_cabinets";
const ERRORS: &str = "popn29_errors";
const COUNTERS: &str = "popn29_counters";
const MUSIC_USAGE: &str = "popn29_music_usage";
const CHARACTER_USAGE: &str = "popn29_character_usage";
const LEGACY_SCORE_INDEX: &str = "player_chart_unique";
const LEGACY_COURSE_INDEX: &str = "player_course_unique";
const USER_PLAY_HISTORY_INDEX: &str = "user_play_history";
const GAME_MODEL: &str = "M39";

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
            .drop_index(SCORES.into(), LEGACY_SCORE_INDEX.into())
            .await?;
        self.raw
            .drop_index(COURSES.into(), LEGACY_COURSE_INDEX.into())
            .await?;
        self.raw
            .create_indexes(
                PLAYERS.into(),
                vec![IndexDefinition {
                    keys: doc! { "g_pm_id": 1 },
                    name: Some("g_pm_id_unique".into()),
                    unique: true,
                    expire_after_seconds: None,
                }],
            )
            .await?;
        self.raw
            .create_indexes(
                SCORES.into(),
                vec![IndexDefinition {
                    keys: doc! { "user_id": 1, "music_num": 1, "sheet_num": 1 },
                    name: Some("user_chart_unique".into()),
                    unique: true,
                    expire_after_seconds: None,
                }],
            )
            .await?;
        self.raw
            .create_indexes(
                ROOMS.into(),
                vec![
                    IndexDefinition {
                        keys: doc! { "location_id": 1, "net_version": 1, "updated_at": -1 },
                        name: Some("matching_list".into()),
                        unique: false,
                        expire_after_seconds: None,
                    },
                    IndexDefinition {
                        keys: doc! { "expires_at": 1 },
                        name: Some("matching_expiry".into()),
                        unique: false,
                        expire_after_seconds: None,
                    },
                ],
            )
            .await?;
        self.raw
            .create_indexes(
                COURSES.into(),
                vec![IndexDefinition {
                    keys: doc! { "user_id": 1, "course_id": 1 },
                    name: Some("user_course_unique".into()),
                    unique: true,
                    expire_after_seconds: None,
                }],
            )
            .await?;
        self.raw
            .create_indexes(
                PLAYS.into(),
                vec![IndexDefinition {
                    keys: doc! { "user_id": 1, "play_id": -1 },
                    name: Some(USER_PLAY_HISTORY_INDEX.into()),
                    unique: false,
                    expire_after_seconds: None,
                }],
            )
            .await?;
        for collection in [MUSIC_USAGE, CHARACTER_USAGE] {
            self.raw
                .create_indexes(
                    collection.into(),
                    vec![IndexDefinition {
                        keys: doc! { "plays": -1, "updated_at": -1, "_id": 1 },
                        name: Some("popularity".into()),
                        unique: false,
                        expire_after_seconds: None,
                    }],
                )
                .await?;
        }
        self.backfill_game_bindings().await?;
        Ok(())
    }

    async fn bind_game_profile(&self, user_id: &str) -> Result<(), String> {
        let now = DateTime::now();
        self.raw
            .update_one(
                GAME_BINDINGS.into(),
                doc! { "_id": format!("{GAME_MODEL}:{user_id}") },
                doc! {
                    "$set": { "user_id": user_id, "model": GAME_MODEL, "updated_at": now },
                    "$setOnInsert": { "created_at": now },
                }
                .into(),
                vibea3::UpdateOptions { upsert: true },
            )
            .await
    }

    async fn backfill_game_bindings(&self) -> Result<(), String> {
        let profiles = self
            .raw
            .find_many(PLAYERS.into(), doc! {}, Default::default())
            .await?;
        for profile in profiles {
            if let Ok(user_id) = profile.get_str("_id") {
                self.bind_game_profile(user_id).await?;
            }
        }
        Ok(())
    }

    async fn user_id(
        &self,
        ref_id: &str,
        claimed_user_id: Option<&str>,
    ) -> Result<Option<String>, String> {
        let session = self
            .raw
            .find_one(
                SESSIONS.into(),
                doc! { "_id": ref_id, "expires_at": { "$gt": DateTime::now() } },
                Default::default(),
            )
            .await?;
        let user_id = session.and_then(|doc| doc.get_str("user_id").ok().map(str::to_owned));
        Ok(user_id.filter(|user_id| claimed_user_id.is_none_or(|claimed| claimed == user_id)))
    }
}

fn decode<T: for<'de> Deserialize<'de>>(document: Document) -> Result<T, String> {
    bson::from_document(document).map_err(error)
}

fn encode<T: Serialize>(value: &T) -> Result<Document, String> {
    bson::to_document(value).map_err(error)
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
