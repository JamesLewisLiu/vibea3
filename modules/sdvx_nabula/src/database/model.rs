use bson::DateTime;
use encoding_rs::SHIFT_JIS;
use serde::{Deserialize, Serialize};

use crate::protocol::{
    INITIAL_APPEAL_ID, INITIAL_BLASTER_ENERGY, INITIAL_GAME_CURRENCY, INITIAL_SKILL_LEVEL,
    INITIAL_SKILL_NAME_ID, OVER_RADAR_ELEMENT_COUNT, PLAYER_CODE_BUFFER_BYTES,
    PLAYER_NAME_BUFFER_BYTES, SDVX_ID_BUFFER_BYTES,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlayerProfile {
    #[serde(rename = "_id")]
    pub user_id: String,
    pub name: String,
    pub code: String,
    pub sdvx_id: String,
    pub appeal_id: u16,
    pub skill_level: i16,
    pub skill_name_id: i16,
    pub skill_type: Option<i16>,
    pub gamecoin_packet: u32,
    pub gamecoin_block: u32,
    pub blaster_energy: u32,
    pub blaster_count: u32,
    pub variant_power: i32,
    pub variant_elements: RadarElements,
    pub over_radar: Vec<i32>,
    pub play_count: u32,
    pub day_count: u32,
    pub today_count: u32,
    pub play_chain: u32,
    pub max_play_chain: u32,
    pub week_count: u32,
    pub week_play_count: u32,
    pub week_chain: u32,
    pub max_week_chain: u32,
    pub creator_id: u32,
    pub eaappli_relation: i8,
    pub blaster_pass_enable: bool,
    pub blaster_pass_limit_date: u64,
    pub kac_id: String,
    pub block_no: i32,
    pub support_team_id: i32,
    pub setting: PlayerSetting,
    pub items: Vec<PlayerItem>,
    pub params: Vec<PlayerParam>,
    pub stories: Vec<StoryProgress>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl PlayerProfile {
    pub(crate) fn new(user_id: String, name: String) -> Self {
        let now = DateTime::now();
        let code = numeric_id(&user_id, PLAYER_CODE_BUFFER_BYTES.saturating_sub(1));
        let sdvx_id = numeric_id(&user_id, SDVX_ID_BUFFER_BYTES.saturating_sub(1));
        Self {
            user_id,
            name: truncate_shift_jis(&name, PLAYER_NAME_BUFFER_BYTES.saturating_sub(1)),
            sdvx_id,
            code,
            appeal_id: INITIAL_APPEAL_ID,
            skill_level: INITIAL_SKILL_LEVEL,
            skill_name_id: INITIAL_SKILL_NAME_ID,
            skill_type: None,
            gamecoin_packet: INITIAL_GAME_CURRENCY,
            gamecoin_block: INITIAL_GAME_CURRENCY,
            blaster_energy: INITIAL_BLASTER_ENERGY,
            blaster_count: 0,
            variant_power: 0,
            variant_elements: RadarElements::default(),
            over_radar: vec![0; OVER_RADAR_ELEMENT_COUNT],
            play_count: 0,
            day_count: 0,
            today_count: 0,
            play_chain: 0,
            max_play_chain: 0,
            week_count: 0,
            week_play_count: 0,
            week_chain: 0,
            max_week_chain: 0,
            creator_id: 0,
            eaappli_relation: 0,
            blaster_pass_enable: false,
            blaster_pass_limit_date: 0,
            kac_id: String::new(),
            block_no: 0,
            support_team_id: 0,
            setting: PlayerSetting::default(),
            items: Vec::new(),
            params: Vec::new(),
            stories: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

fn numeric_id(user_id: &str, capacity: usize) -> String {
    let digits: String = user_id.chars().filter(char::is_ascii_digit).collect();
    let suffix = if digits.len() > capacity {
        &digits[digits.len() - capacity..]
    } else {
        &digits
    };
    format!("{suffix:0>capacity$}")
}

fn truncate_shift_jis(value: &str, maximum: usize) -> String {
    let mut result = String::new();
    let mut used = 0;
    for character in value.chars() {
        let text = character.to_string();
        let (encoded, _, had_errors) = SHIFT_JIS.encode(&text);
        if had_errors || used + encoded.len() > maximum {
            break;
        }
        result.push(character);
        used += encoded.len();
    }
    result
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct RadarElements {
    pub notes: i32,
    pub peak: i32,
    pub tsumami: i32,
    pub tricky: i32,
    pub onehand: i32,
    pub handtrip: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct PlayerSetting {
    pub hispeed: i32,
    pub lanespeed: u32,
    pub gauge_option: u8,
    pub ars_option: u8,
    pub notes_option: u8,
    pub early_late_disp: u8,
    pub draw_adjust: i32,
    pub eff_c_left: u8,
    pub eff_c_right: u8,
    pub music_id: i32,
    pub music_type: u8,
    pub sort_type: u8,
    pub narrow_down: u8,
    pub headphone: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlayerItem {
    pub item_type: u32,
    pub id: u32,
    pub param: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlayerParam {
    pub param_type: i32,
    #[serde(default)]
    pub id: i32,
    pub values: Vec<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StoryProgress {
    pub story_id: i32,
    pub progress_id: i32,
    pub progress_param: i32,
    pub clear_cnt: i32,
    pub route_flg: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MusicScore {
    #[serde(rename = "_id")]
    pub id: String,
    pub user_id: String,
    pub music_id: u32,
    pub music_type: u32,
    pub score: u32,
    pub exscore: u32,
    pub clear_type: u32,
    pub score_grade: u32,
    pub max_chain: u32,
    pub best_critical: u32,
    pub best_near: u32,
    pub best_error: u32,
    pub volforce: u32,
    pub just: u32,
    pub effective_rate: u32,
    pub btn_rate: u32,
    pub long_rate: u32,
    pub vol_rate: u32,
    pub mode: u8,
    pub start_option: u8,
    pub gauge_type: u8,
    pub notes_option: u8,
    pub online_num: u16,
    pub local_num: u16,
    pub challenge_type: u8,
    pub retry_cnt: i32,
    pub judge: Vec<i32>,
    pub mix_id: i32,
    pub mix_like: bool,
    pub matching: Vec<MatchingResult>,
    pub play_count: u32,
    #[serde(default)]
    pub clear_count: u32,
    #[serde(default)]
    pub ultimate_chain_count: u32,
    #[serde(default)]
    pub perfect_ultimate_chain_count: u32,
    pub updated_at: DateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MatchingResult {
    pub code: String,
    pub score: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlayRecord {
    #[serde(rename = "_id")]
    pub play_id: i64,
    pub user_id: String,
    pub location_id: String,
    pub started_at: DateTime,
    pub ended_at: Option<DateTime>,
}
