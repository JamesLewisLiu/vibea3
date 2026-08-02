use bson::{DateTime, doc};
use serde::{Deserialize, Serialize};
use vibea3::{FindOptions, UpdateOptions};

use super::{Database, ROOMS, decode, encode};
use crate::protocol::{DEFAULT_LOBBY_LIFETIME_MILLIS, LOBBY_PLAYER_CAPACITY, LOBBY_ROOM_CAPACITY};

#[derive(Clone, Debug)]
pub(crate) struct LobbyEntry {
    pub tag: String,
    pub ip: u32,
    pub local_ip: u32,
    pub time: u32,
    pub port: u16,
    pub music: i16,
    pub sheet: u8,
    pub is_ojama: u8,
    pub location_id: String,
    pub net_version: u8,
    pub gpm_id: String,
    pub staff: i8,
    pub item_type: i16,
    pub item_id: i16,
    pub is_random: i8,
    pub license_data: Vec<i16>,
    pub is_ranking: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LobbyRoom {
    #[serde(rename = "_id")]
    pub no: u32,
    #[serde(default)]
    pub tag: String,
    pub ip: u32,
    pub local_ip: u32,
    pub time: u32,
    pub port: u16,
    pub music: i16,
    pub sheet: u8,
    pub is_ojama: u8,
    pub location_id: String,
    pub net_version: u8,
    pub gpm_id: String,
    #[serde(default)]
    pub gpm_ids: Vec<String>,
    pub matched_cnt: u8,
    pub staff: i8,
    pub item_type: i16,
    pub item_id: i16,
    pub is_random: i8,
    pub license_data: Vec<i16>,
    pub is_ranking: i8,
    pub updated_at: DateTime,
    pub expires_at: DateTime,
}

impl LobbyRoom {
    pub(crate) fn gpm_ids(&self) -> Vec<String> {
        if self.gpm_ids.is_empty() && !self.gpm_id.is_empty() {
            vec![self.gpm_id.clone()]
        } else {
            self.gpm_ids.clone()
        }
    }
}

impl Database {
    pub(crate) async fn enter_lobby(&self, entry: LobbyEntry) -> Result<u32, String> {
        let no = self.next_lobby_id().await?;
        let now = DateTime::now();
        let expires_at = DateTime::from_millis(now.timestamp_millis() + i64::from(entry.time));
        let room = LobbyRoom {
            no,
            tag: entry.tag,
            ip: entry.ip,
            local_ip: entry.local_ip,
            time: entry.time,
            port: entry.port,
            music: entry.music,
            sheet: entry.sheet,
            is_ojama: entry.is_ojama,
            location_id: entry.location_id,
            net_version: entry.net_version,
            gpm_ids: vec![entry.gpm_id.clone()],
            gpm_id: entry.gpm_id,
            matched_cnt: 0,
            staff: entry.staff,
            item_type: entry.item_type,
            item_id: entry.item_id,
            is_random: entry.is_random,
            license_data: entry.license_data,
            is_ranking: entry.is_ranking,
            updated_at: now,
            expires_at,
        };
        self.raw.insert_one(ROOMS.into(), encode(&room)?).await?;
        Ok(no)
    }

    pub(crate) async fn lobby_rooms(
        &self,
        location_id: &str,
        net_version: u8,
        exclude_tag: Option<&str>,
    ) -> Result<Vec<LobbyRoom>, String> {
        self.raw
            .delete_many(
                ROOMS.into(),
                doc! { "expires_at": { "$lte": DateTime::now() } },
            )
            .await?;
        let mut filter = doc! {
            "location_id": location_id,
            "net_version": i32::from(net_version),
            "expires_at": { "$gt": DateTime::now() },
        };
        if let Some(tag) = exclude_tag.filter(|tag| !tag.is_empty()) {
            filter.insert("tag", doc! { "$ne": tag });
        }
        self.raw
            .find_many(
                ROOMS.into(),
                filter,
                FindOptions {
                    sort: Some(doc! { "updated_at": -1 }),
                    limit: Some(LOBBY_ROOM_CAPACITY),
                },
            )
            .await?
            .into_iter()
            .map(decode)
            .collect()
    }

    pub(crate) async fn update_lobby(
        &self,
        no: u32,
        matched_cnt: u8,
        location_id: &str,
        gpm_id: &str,
        staff: i8,
    ) -> Result<(), String> {
        let now = DateTime::now();
        let room = self
            .raw
            .find_one(
                ROOMS.into(),
                doc! { "_id": i64::from(no) },
                Default::default(),
            )
            .await?;
        let lifetime = room
            .as_ref()
            .and_then(|room| room.get_i64("time").ok())
            .or_else(|| {
                room.as_ref()
                    .and_then(|room| room.get_i32("time").ok().map(i64::from))
            })
            .unwrap_or(DEFAULT_LOBBY_LIFETIME_MILLIS)
            .max(0);
        self.raw
            .update_one(
                ROOMS.into(),
                doc! { "_id": i64::from(no), "location_id": location_id },
                vec![doc! { "$set": {
                    "matched_cnt": i32::from(matched_cnt),
                    "gpm_id": gpm_id,
                    "gpm_ids": { "$slice": [
                        { "$setUnion": [
                            { "$ifNull": ["$gpm_ids", ["$gpm_id"]] },
                            [gpm_id],
                        ] },
                        i64::try_from(LOBBY_PLAYER_CAPACITY).expect("lobby capacity fits i64"),
                    ] },
                    "staff": i32::from(staff),
                    "updated_at": now,
                    "expires_at": DateTime::from_millis(now.timestamp_millis() + lifetime),
                }}]
                .into(),
                UpdateOptions::default(),
            )
            .await
    }

    pub(crate) async fn delete_lobby(&self, no: u32) -> Result<(), String> {
        self.raw
            .delete_one(ROOMS.into(), doc! { "_id": i64::from(no) })
            .await
    }
}
