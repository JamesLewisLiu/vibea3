use bson::DateTime;
use serde::{Deserialize, Serialize};

use crate::protocol::{
    CUSTOMIZE_FIELD_COUNT, FAVORITE_CHARACTER_HISTORY_COUNT, LATEST_MUSIC_HISTORY_COUNT,
    NICE_HISTORY_COUNT, POWER_POINT_HISTORY_COUNT,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlayerProfile {
    #[serde(rename = "_id")]
    pub user_id: String,
    pub g_pm_id: String,
    pub name: String,
    pub pref: i8,
    #[serde(default)]
    pub tutorial: i16,
    #[serde(default)]
    pub read_news: i16,
    #[serde(default)]
    pub total_play_cnt: i16,
    #[serde(default)]
    pub latest_music: Vec<i16>,
    #[serde(default)]
    pub nice: Vec<i16>,
    #[serde(default)]
    pub favorite_chara: Vec<i16>,
    #[serde(default = "default_negative_one_i8")]
    pub popn_class: i8,
    #[serde(default)]
    pub power_point: i32,
    #[serde(default)]
    pub power_point_list: Vec<i32>,
    #[serde(default = "default_negative_one_i32")]
    pub sc_news_no: i32,
    #[serde(default)]
    pub read_policy: i16,
    #[serde(default = "default_negative_one_i8")]
    pub language: i8,
    #[serde(default)]
    pub eaappli_relation: i8,
    #[serde(default)]
    pub ep: u16,
    #[serde(default)]
    pub estatus: u16,
    #[serde(default)]
    pub customize: Vec<u16>,
    #[serde(default)]
    pub option: MusicOption,
    #[serde(default)]
    pub config: ConfigState,
    #[serde(default)]
    pub items: Vec<ItemState>,
    #[serde(default)]
    pub characters: Vec<CharacterState>,
    #[serde(default)]
    pub extra: Vec<ExtraState>,
    #[serde(default)]
    pub netvs: NetvsState,
    #[serde(default)]
    pub event: EventState,
    #[serde(default)]
    pub lumina: i32,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl PlayerProfile {
    pub(crate) fn new(user_id: String, g_pm_id: String, name: String, pref: i8) -> Self {
        let now = DateTime::now();
        Self {
            user_id,
            g_pm_id,
            name,
            pref,
            tutorial: 0,
            read_news: 0,
            total_play_cnt: 0,
            latest_music: vec![-1; LATEST_MUSIC_HISTORY_COUNT],
            nice: vec![-1; NICE_HISTORY_COUNT],
            favorite_chara: vec![-1; FAVORITE_CHARACTER_HISTORY_COUNT],
            popn_class: -1,
            power_point: 0,
            power_point_list: vec![-1; POWER_POINT_HISTORY_COUNT],
            sc_news_no: -1,
            read_policy: 0,
            language: -1,
            eaappli_relation: 0,
            ep: 0,
            estatus: 0,
            customize: vec![0; CUSTOMIZE_FIELD_COUNT],
            option: MusicOption::default(),
            config: ConfigState::default(),
            items: Vec::new(),
            characters: Vec::new(),
            extra: Vec::new(),
            netvs: NetvsState::default(),
            event: EventState::default(),
            lumina: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ProfilePatch {
    pub tutorial: Option<i16>,
    pub read_news: Option<i16>,
    pub latest_music: Option<Vec<i16>>,
    pub nice: Option<Vec<i16>>,
    pub favorite_chara: Option<Vec<i16>>,
    pub popn_class: Option<i8>,
    pub power_point: Option<i32>,
    pub power_point_list: Option<Vec<i32>>,
    pub sc_news_no: Option<i32>,
    pub read_policy: Option<i16>,
    pub language: Option<i8>,
    pub ep: Option<u16>,
    pub estatus: Option<u16>,
    pub customize: Option<Vec<u16>>,
    pub option: Option<MusicOption>,
    pub config: Option<ConfigState>,
    pub items: Option<Vec<ItemState>>,
    pub characters: Option<Vec<CharacterState>>,
    pub extra: Option<Vec<ExtraState>>,
    pub netvs: Option<NetvsState>,
    pub event: Option<EventState>,
}

fn default_negative_one_i8() -> i8 {
    -1
}

fn default_negative_one_i32() -> i32 {
    -1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ConfigState {
    pub mode: u8,
    pub chara: i16,
    pub music: i16,
    pub sheet: u8,
    pub category: i8,
    pub sub_category: i8,
    pub chara_category: i8,
    pub ms_banner_disp: i8,
    pub ms_down_info: i8,
    pub ms_raise_type: i8,
    pub banner_sort: i8,
    pub disp_setting: i8,
    pub h_vol: i8,
    pub hiscore_disp: bool,
    pub lane_type: i8,
    pub brightness: i8,
    pub key_beam: i8,
    pub lane_line: i8,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            mode: 0,
            chara: 0,
            music: 0,
            sheet: 0,
            category: 0,
            sub_category: 0,
            chara_category: 0,
            ms_banner_disp: 0,
            ms_down_info: 0,
            ms_raise_type: 0,
            banner_sort: 0,
            disp_setting: 0,
            h_vol: 1,
            hiscore_disp: false,
            lane_type: 0,
            brightness: 100,
            key_beam: 100,
            lane_line: 1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ItemState {
    pub item_type: u8,
    pub id: u16,
    pub param: u16,
    pub is_new: bool,
    pub get_time: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CharacterState {
    pub chara_id: u16,
    pub friendship: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ExtraState {
    pub music_num: i16,
    pub point: u8,
    pub ex_gauge_lv: Vec<u8>,
    pub clear_lv: Vec<u8>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct NetvsState {
    pub record: Vec<i16>,
    pub dialogs: Vec<String>,
    pub ojama_condition: Vec<i8>,
    pub set_ojama: Vec<i8>,
    pub set_recommend: Vec<i8>,
    pub rival_ids: Vec<String>,
    pub select_num_normal: u32,
    pub select_num_ojama: u32,
    pub netvs_play_cnt: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct EventState {
    pub basket_id: Option<i16>,
    #[serde(default)]
    pub baskets: Vec<EventBasket>,
    pub ensta_checked: Option<bool>,
    pub ensta_serial_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EventBasket {
    pub id: i16,
    pub point: u32,
    pub is_cleared: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MusicOption {
    pub hispeed: i16,
    pub popkun: u8,
    pub hidden: bool,
    pub hidden_rate: i16,
    pub sudden: bool,
    pub sudden_rate: i16,
    pub randmir: i8,
    pub gauge_type: i8,
    pub ojama_0: u8,
    pub ojama_1: u8,
    pub forever_0: bool,
    pub forever_1: bool,
    pub full_setting: bool,
    pub judge: u8,
    pub guide_se: i8,
    pub guide_se_vol: u8,
    pub lift: bool,
    pub lift_rate: i16,
    pub judge_ad: i8,
    pub roof: i16,
    pub long_pop: i8,
}

impl Default for MusicOption {
    fn default() -> Self {
        Self {
            hispeed: 0,
            popkun: 0,
            hidden: false,
            hidden_rate: -70,
            sudden: false,
            sudden_rate: -270,
            randmir: 0,
            gauge_type: 0,
            ojama_0: 0,
            ojama_1: 0,
            forever_0: false,
            forever_1: false,
            full_setting: false,
            judge: 0,
            guide_se: 0,
            guide_se_vol: 3,
            lift: false,
            lift_rate: 0,
            judge_ad: 0,
            roof: 0,
            long_pop: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MusicScore {
    #[serde(rename = "_id")]
    pub id: String,
    pub user_id: String,
    pub music_num: i16,
    pub sheet_num: u8,
    pub score: i32,
    pub clear_type: u8,
    pub clear_rank: u8,
    pub cnt: i16,
    #[serde(default)]
    pub score_ver: i32,
    #[serde(default)]
    pub clear_type_ver: u8,
    #[serde(default)]
    pub option: Option<MusicOption>,
    pub updated_at: DateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct UsageCounter {
    #[serde(rename = "_id")]
    pub id: i32,
    pub plays: i64,
    pub updated_at: DateTime,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct UsageSnapshot {
    pub popular_characters: Vec<i16>,
    pub popular_music: Vec<i16>,
    pub recommend_music: Vec<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CourseRecord {
    #[serde(rename = "_id")]
    pub id: String,
    pub user_id: String,
    pub pref: i16,
    pub location_id: String,
    pub data_id: String,
    pub name: String,
    pub chara_num: i16,
    pub play_id: i32,
    pub course_id: i16,
    pub course_name: String,
    pub stage1_music_num: i16,
    pub stage1_sheet_num: u8,
    pub stage2_music_num: i16,
    pub stage2_sheet_num: u8,
    pub stage3_music_num: i16,
    pub stage3_sheet_num: u8,
    pub stage4_music_num: i16,
    pub stage4_sheet_num: u8,
    pub norma_type: u8,
    pub norma_1_num: i32,
    pub norma_2_num: i32,
    pub clear_medal: u8,
    pub clear_norma: u8,
    pub total_score: i32,
    pub max_combo: i16,
    pub last_gauge: i16,
    pub is_image_store: bool,
    pub is_license: bool,
    pub license_data: Vec<i16>,
    pub updated_at: DateTime,
}
