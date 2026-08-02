use bson::{DateTime, doc, oid::ObjectId};
use serde::{Deserialize, Serialize};
use vibea3::{FindOneAndUpdateOptions, FindOneOptions, ReturnDocument};

use super::{CARDS, Database, GAME_BINDINGS, SESSIONS, USERS, decode, encode};
use crate::{REF_ID_DIGIT_COUNT, USER_ID_DIGIT_COUNT};

const CARD_SESSION_TTL_SECONDS: u64 = 24 * 60 * 60;
const MILLISECONDS_PER_SECOND: u64 = 1_000;

impl Database {
    pub(crate) async fn inquire_card(
        &self,
        card_id: String,
        model: String,
        tag: String,
        update: bool,
    ) -> Result<CardInquiry, String> {
        let Some(card) = self
            .raw
            .find_one(CARDS.into(), doc! { "_id": &card_id }, Default::default())
            .await?
            .map(decode)
            .transpose()?
        else {
            return Ok(CardInquiry::Missing);
        };
        let card: CardDocument = card;
        if card.banned {
            return Ok(CardInquiry::Banned);
        }
        if !card.active {
            return Ok(CardInquiry::Inactive);
        }
        let user_id = match card.user_id {
            Some(user_id) => user_id,
            None => self.assign_user(&card_id).await?,
        };
        self.ensure_user(&user_id).await?;
        let ref_id = if update {
            match self
                .raw
                .find_one(
                    SESSIONS.into(),
                    doc! {
                        "card_id": &card_id,
                        "logged_in": true,
                        "expires_at": { "$gt": DateTime::now() },
                    },
                    FindOneOptions {
                        sort: Some(doc! { "updated_at": -1 }),
                    },
                )
                .await?
                .map(decode)
                .transpose()?
            {
                Some(session) => {
                    let session: SessionDocument = session;
                    self.touch_session(&session.ref_id).await?;
                    session.ref_id
                }
                None => self.create_session(&card_id, &user_id, &tag, false).await?,
            }
        } else {
            self.create_session(&card_id, &user_id, &tag, false).await?
        };
        let prefix = model.split(':').next().unwrap_or_default();
        let bound = self
            .raw
            .find_one(
                GAME_BINDINGS.into(),
                doc! { "_id": binding_id(prefix, &user_id) },
                Default::default(),
            )
            .await?
            .is_some();
        Ok(CardInquiry::Active {
            ref_id,
            user_id,
            eacoin_enabled: card.eacoin_enabled,
            bound,
        })
    }

    pub(crate) async fn register_card(
        &self,
        card_id: String,
        pin: String,
        tag: String,
    ) -> Result<Option<CardIdentity>, String> {
        let existing = self
            .raw
            .find_one(CARDS.into(), doc! { "_id": &card_id }, Default::default())
            .await?
            .map(decode::<CardDocument>)
            .transpose()?;
        if existing.as_ref().is_some_and(|card| card.banned) {
            return Ok(None);
        }
        if existing.as_ref().is_some_and(|card| card.active) {
            return Ok(None);
        }
        let existing_user_id = existing.and_then(|card| card.user_id);
        let candidate_user_id = existing_user_id
            .clone()
            .unwrap_or_else(|| decimal_id(USER_ID_DIGIT_COUNT));
        let card = self
            .raw
            .find_one_and_update(
                CARDS.into(),
                doc! { "_id": &card_id },
                doc! {
                    "$set": { "pin": pin, "active": true },
                    "$setOnInsert": {
                        "user_id": &candidate_user_id,
                        "banned": false,
                        "eacoin_enabled": true,
                        "balance": 0,
                        "auto_charge_enabled": false,
                        "auto_charge_threshold": 0,
                        "auto_charge_amount": 0,
                    }
                }
                .into(),
                FindOneAndUpdateOptions {
                    upsert: true,
                    return_document: ReturnDocument::After,
                },
            )
            .await?;
        let Some(card) = card.map(decode::<CardDocument>).transpose()? else {
            return Ok(None);
        };
        let user_id = match card.user_id.or(existing_user_id) {
            Some(user_id) => user_id,
            None => self.assign_user(&card_id).await?,
        };
        self.ensure_user(&user_id).await?;
        let ref_id = self.create_session(&card_id, &user_id, &tag, true).await?;
        Ok(Some(CardIdentity { ref_id, user_id }))
    }

    pub(crate) async fn session_user(&self, ref_id: String) -> Result<Option<String>, String> {
        let session = self
            .raw
            .find_one(
                SESSIONS.into(),
                doc! {
                    "_id": ref_id,
                    "logged_in": true,
                    "expires_at": { "$gt": DateTime::now() },
                },
                Default::default(),
            )
            .await?
            .map(decode::<SessionDocument>)
            .transpose()?;
        Ok(session.map(|session| session.user_id))
    }

    pub(crate) async fn authenticate(
        &self,
        ref_id: String,
        pin: String,
    ) -> Result<AuthResult, String> {
        let Some(session) = self
            .raw
            .find_one(
                SESSIONS.into(),
                doc! { "_id": &ref_id, "expires_at": { "$gt": DateTime::now() } },
                Default::default(),
            )
            .await?
            .map(decode)
            .transpose()?
        else {
            return Ok(AuthResult::Missing);
        };
        let session: SessionDocument = session;
        let Some(card) = self
            .raw
            .find_one(
                CARDS.into(),
                doc! { "_id": &session.card_id, "active": true },
                Default::default(),
            )
            .await?
            .map(decode)
            .transpose()?
        else {
            return Ok(AuthResult::Missing);
        };
        let card: CardDocument = card;
        if card.pin.as_deref() != Some(pin.as_str()) {
            return Ok(AuthResult::Invalid);
        }
        self.raw
            .update_one(
                SESSIONS.into(),
                doc! { "_id": &ref_id },
                doc! { "$set": {
                    "logged_in": true,
                    "updated_at": DateTime::now(),
                    "expires_at": session_expiry(),
                } }
                .into(),
                Default::default(),
            )
            .await?;
        Ok(AuthResult::Valid)
    }

    async fn assign_user(&self, card_id: &str) -> Result<String, String> {
        let user_id = decimal_id(USER_ID_DIGIT_COUNT);
        self.ensure_user(&user_id).await?;
        self.raw
            .update_one(
                CARDS.into(),
                doc! { "_id": card_id },
                doc! { "$set": { "user_id": &user_id } }.into(),
                Default::default(),
            )
            .await?;
        Ok(user_id)
    }

    async fn ensure_user(&self, user_id: &str) -> Result<(), String> {
        let now = DateTime::now();
        self.raw
            .update_one(
                USERS.into(),
                doc! { "_id": user_id },
                doc! {
                    "$set": { "updated_at": now },
                    "$setOnInsert": { "created_at": now },
                }
                .into(),
                vibea3::UpdateOptions { upsert: true },
            )
            .await
    }

    async fn touch_session(&self, ref_id: &str) -> Result<(), String> {
        self.raw
            .update_one(
                SESSIONS.into(),
                doc! { "_id": ref_id },
                doc! { "$set": {
                    "updated_at": DateTime::now(),
                    "expires_at": session_expiry(),
                } }
                .into(),
                Default::default(),
            )
            .await
    }

    async fn create_session(
        &self,
        card_id: &str,
        user_id: &str,
        tag: &str,
        logged_in: bool,
    ) -> Result<String, String> {
        let ref_id = decimal_id(REF_ID_DIGIT_COUNT);
        self.raw
            .insert_one(
                SESSIONS.into(),
                encode(&SessionDocument {
                    ref_id: ref_id.clone(),
                    card_id: card_id.to_owned(),
                    user_id: user_id.to_owned(),
                    tag: tag.to_owned(),
                    logged_in,
                    updated_at: DateTime::now(),
                    expires_at: session_expiry(),
                })?,
            )
            .await?;
        Ok(ref_id)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum CardInquiry {
    Missing,
    Banned,
    Inactive,
    Active {
        ref_id: String,
        user_id: String,
        eacoin_enabled: bool,
        bound: bool,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct CardIdentity {
    pub(crate) ref_id: String,
    pub(crate) user_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthResult {
    Valid,
    Invalid,
    Missing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CardDocument {
    #[serde(rename = "_id")]
    card_id: String,
    #[serde(default)]
    pub(super) user_id: Option<String>,
    #[serde(default)]
    pub(super) pin: Option<String>,
    #[serde(default)]
    pub(super) active: bool,
    #[serde(default)]
    pub(super) banned: bool,
    #[serde(default)]
    pub(super) eacoin_enabled: bool,
    #[serde(default)]
    pub(super) balance: i32,
    #[serde(default)]
    pub(super) auto_charge_enabled: bool,
    #[serde(default)]
    auto_charge_threshold: i32,
    #[serde(default)]
    auto_charge_amount: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SessionDocument {
    #[serde(rename = "_id")]
    ref_id: String,
    card_id: String,
    user_id: String,
    tag: String,
    logged_in: bool,
    updated_at: DateTime,
    #[serde(default = "session_expiry")]
    expires_at: DateTime,
}

fn decimal_id(length: usize) -> String {
    let value = ObjectId::new()
        .bytes()
        .into_iter()
        .fold(0_u128, |value, byte| (value << 8) | u128::from(byte));
    let modulus = 10_u128.pow(u32::try_from(length).expect("identity length fits u32"));
    format!("{:0width$}", value % modulus, width = length)
}

fn binding_id(model: &str, user_id: &str) -> String {
    format!("{model}:{user_id}")
}

fn session_expiry() -> DateTime {
    let ttl_millis = i64::try_from(CARD_SESSION_TTL_SECONDS * MILLISECONDS_PER_SECOND)
        .expect("card session TTL fits i64 milliseconds");
    DateTime::from_millis(DateTime::now().timestamp_millis() + ttl_millis)
}
