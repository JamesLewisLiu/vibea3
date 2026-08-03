//! Wire constants recovered from the KFC 2026-07-14 `sv7.dll` client.
//!
//! Counts here are fixed client buffers or psmap array widths, never catalog
//! cardinalities or maximum IDs.

pub(crate) const OVER_RADAR_ELEMENT_COUNT: usize = 6;
pub(crate) const RIVAL_MUSIC_PARAM_COUNT: usize = 6;
pub(crate) const MUSIC_RECORD_PARAM_COUNT: usize = 26;
pub(crate) const MUSIC_PARAM_MUSIC_ID: usize = 0;
pub(crate) const MUSIC_PARAM_MUSIC_TYPE: usize = 1;
pub(crate) const MUSIC_PARAM_PRIMARY_SCORE: usize = 2;
pub(crate) const MUSIC_PARAM_PRIMARY_EXSCORE: usize = 3;
pub(crate) const MUSIC_PARAM_PRIMARY_CLEAR_TYPE: usize = 4;
pub(crate) const MUSIC_PARAM_PRIMARY_GRADE: usize = 5;
pub(crate) const MUSIC_PARAM_PRIMARY_MAX_CHAIN: usize = 6;
pub(crate) const MUSIC_PARAM_PRIMARY_PLAY_COUNT: usize = 7;
pub(crate) const MUSIC_PARAM_PRIMARY_CLEAR_COUNT: usize = 8;
pub(crate) const MUSIC_PARAM_PRIMARY_UC_COUNT: usize = 9;
pub(crate) const MUSIC_PARAM_PRIMARY_PUC_COUNT: usize = 10;
pub(crate) const MUSIC_PARAM_VOLFORCE: usize = 11;
pub(crate) const MUSIC_PARAM_BUTTON_RATE: usize = 23;
pub(crate) const MUSIC_PARAM_LONG_RATE: usize = 24;
pub(crate) const MUSIC_PARAM_VOL_RATE: usize = 25;
pub(crate) const TRACK_JUDGE_COUNT: usize = 7;
pub(crate) const TRACK_MATCHING_PLAYER_COUNT: usize = 3;
pub(crate) const PARAMETER_VALUE_COUNT: usize = 256;
pub(crate) const CLEAR_TYPE_CLEAR: u32 = 2;
pub(crate) const CLEAR_TYPE_ULTIMATE_CHAIN: u32 = 5;
pub(crate) const CLEAR_TYPE_PERFECT_ULTIMATE_CHAIN: u32 = 6;
#[allow(dead_code)]
pub(crate) const ARENA_RANK_TARGET_COUNT: usize = 32;
pub(crate) const HISCORE_LEVEL_BUCKET_COUNT: usize = 13;
pub(crate) const HISCORE_PAGE_DEFAULT: u32 = 20;
pub(crate) const DEFAULT_LOUNGE_INTERVAL_SECONDS: u32 = 10;
pub(crate) const DEFAULT_SHOP_NEXT_TIME_SECONDS: u32 = 1_800;
pub(crate) const INITIAL_GAME_CURRENCY: u32 = 0;
pub(crate) const INITIAL_BLASTER_ENERGY: u32 = 0;
pub(crate) const INITIAL_APPEAL_ID: u16 = 1;
pub(crate) const INITIAL_SKILL_LEVEL: i16 = 0;
pub(crate) const INITIAL_SKILL_NAME_ID: i16 = 0;
pub(crate) const CURRENCY_GAMECOIN_PACKET: u32 = 0;
pub(crate) const CURRENCY_GAMECOIN_BLOCK: u32 = 1;
pub(crate) const BUY_RESULT_SUCCESS: i8 = 0;
pub(crate) const BUY_RESULT_INSUFFICIENT_FUNDS: i8 = 1;
pub(crate) const PLAYER_NAME_BUFFER_BYTES: usize = 9;
pub(crate) const PLAYER_CODE_BUFFER_BYTES: usize = 17;
pub(crate) const SDVX_ID_BUFFER_BYTES: usize = 13;
pub(crate) const GAME_MODEL: &str = "KFC";
