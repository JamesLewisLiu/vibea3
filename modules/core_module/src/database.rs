mod card;
mod eacoin;
mod machine;

use std::sync::Arc;

use bson::{Document, doc};
use serde::{Deserialize, Serialize};
use vibea3::{DocumentDatabase, IndexDefinition};

pub(crate) use card::{AuthResult, CardInquiry};
pub(crate) use eacoin::ConsumeInput;
pub(crate) use machine::FacilityInfo;

const MACHINES: &str = "machines";
const USERS: &str = "users";
const CARDS: &str = "cards";
const SESSIONS: &str = "card_sessions";
const GAME_BINDINGS: &str = "game_bindings";
const LEDGER: &str = "eacoin_ledger";
const USER_MODEL_BINDING_INDEX: &str = "user_model_binding";

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
            .delete_many(SESSIONS.into(), doc! { "expires_at": { "$exists": false } })
            .await?;
        self.raw
            .create_indexes(
                CARDS.into(),
                vec![IndexDefinition {
                    keys: doc! { "user_id": 1 },
                    name: Some("user_cards".into()),
                    unique: false,
                    expire_after_seconds: None,
                }],
            )
            .await?;
        self.raw
            .create_indexes(
                SESSIONS.into(),
                vec![
                    IndexDefinition {
                        keys: doc! { "card_id": 1, "updated_at": -1 },
                        name: Some("card_updated".into()),
                        unique: false,
                        expire_after_seconds: None,
                    },
                    IndexDefinition {
                        keys: doc! { "user_id": 1, "updated_at": -1 },
                        name: Some("user_session_updated".into()),
                        unique: false,
                        expire_after_seconds: None,
                    },
                    IndexDefinition {
                        keys: doc! { "expires_at": 1 },
                        name: Some("session_expiry".into()),
                        unique: false,
                        expire_after_seconds: Some(0),
                    },
                ],
            )
            .await?;
        self.raw
            .create_indexes(
                GAME_BINDINGS.into(),
                vec![IndexDefinition {
                    keys: doc! { "user_id": 1, "model": 1 },
                    name: Some(USER_MODEL_BINDING_INDEX.into()),
                    unique: true,
                    expire_after_seconds: None,
                }],
            )
            .await?;
        self.raw
            .create_indexes(
                LEDGER.into(),
                vec![IndexDefinition {
                    keys: doc! { "card_id": 1, "created_at": -1 },
                    name: Some("card_created".into()),
                    unique: false,
                    expire_after_seconds: None,
                }],
            )
            .await
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
