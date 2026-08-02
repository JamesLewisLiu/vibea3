//! Fixed M39 wire/destination capacities audited from `popn.dll`.
//!
//! These values describe fixed packet fields or client buffers. They must not
//! be used as catalog cardinalities or upper ID bounds.

pub(crate) const PLAYABLE_SHEET_COUNT: usize = 7;
pub(crate) const CLEAR_MEDAL_TYPE_COUNT: i8 = 13;
pub(crate) const LATEST_MUSIC_HISTORY_COUNT: usize = 30;
pub(crate) const NICE_HISTORY_COUNT: usize = 100;
pub(crate) const FAVORITE_CHARACTER_HISTORY_COUNT: usize = 100;
pub(crate) const POWER_POINT_HISTORY_COUNT: usize = 20;
pub(crate) const CUSTOMIZE_FIELD_COUNT: usize = 12;
pub(crate) const EXTRA_LEVEL_COUNT: usize = 4;
pub(crate) const NETVS_RECORD_COUNT: usize = 6;
pub(crate) const NETVS_DIALOG_COUNT: usize = 6;
pub(crate) const NETVS_OJAMA_CONDITION_COUNT: usize = 74;
pub(crate) const NETVS_SET_COUNT: usize = 3;
pub(crate) const POPULAR_MUSIC_CAPACITY: u64 = 500;
pub(crate) const POPULAR_CHARACTER_CAPACITY: u64 = 20;
pub(crate) const RECOMMENDATION_COUNT: usize = 30;
pub(crate) const LOBBY_ROOM_CAPACITY: u64 = 30;
pub(crate) const LOBBY_PLAYER_CAPACITY: usize = 6;
// M39 reads/writes 40 bytes for these s16 arrays: 20 elements, not 10.
pub(crate) const LOBBY_LICENSE_DATA_COUNT: usize = 20;
pub(crate) const COURSE_LICENSE_DATA_COUNT: usize = 20;
pub(crate) const FIXED_PLAYER_TEXT_BYTES: usize = 13;
pub(crate) const FIXED_PLAYER_TEXT_PAYLOAD_BYTES: usize = FIXED_PLAYER_TEXT_BYTES - 1;
pub(crate) const NEWS_TITLE_BYTES: usize = 45;
pub(crate) const NEWS_BODY_BYTES: usize = 1024;
pub(crate) const MUSIC_TEXT_BYTES: usize = 128;
pub(crate) const AUXILIARY_TEXT_BYTES: usize = 64;
pub(crate) const MUSIC_TAG_COUNT: usize = 32;
pub(crate) const CHARACTER_BATTLE_BITMAP_COUNT: usize = 2;
pub(crate) const DEFAULT_LOBBY_LIFETIME_MILLIS: i64 = 120_000;
