use vibea3::Kbin;

use crate::protocol::{
    EXTRA_LEVEL_COUNT, NETVS_OJAMA_CONDITION_COUNT, NETVS_RECORD_COUNT, NETVS_SET_COUNT,
};

use crate::database::{
    CharacterState, ConfigState, ExtraState, ItemState, MusicOption, NetvsState,
};

#[derive(Kbin)]
#[kbin(node = "option")]
pub(super) struct OptionData {
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

impl From<MusicOption> for OptionData {
    fn from(value: MusicOption) -> Self {
        Self {
            hispeed: value.hispeed,
            popkun: value.popkun,
            hidden: value.hidden,
            hidden_rate: value.hidden_rate,
            sudden: value.sudden,
            sudden_rate: value.sudden_rate,
            randmir: value.randmir,
            gauge_type: value.gauge_type,
            ojama_0: value.ojama_0,
            ojama_1: value.ojama_1,
            forever_0: value.forever_0,
            forever_1: value.forever_1,
            full_setting: value.full_setting,
            judge: value.judge,
            guide_se: value.guide_se,
            guide_se_vol: value.guide_se_vol,
            lift: value.lift,
            lift_rate: value.lift_rate,
            judge_ad: value.judge_ad,
            roof: value.roof,
            long_pop: value.long_pop,
        }
    }
}

impl From<OptionData> for MusicOption {
    fn from(value: OptionData) -> Self {
        Self {
            hispeed: value.hispeed,
            popkun: value.popkun,
            hidden: value.hidden,
            hidden_rate: value.hidden_rate,
            sudden: value.sudden,
            sudden_rate: value.sudden_rate,
            randmir: value.randmir,
            gauge_type: value.gauge_type,
            ojama_0: value.ojama_0,
            ojama_1: value.ojama_1,
            forever_0: value.forever_0,
            forever_1: value.forever_1,
            full_setting: value.full_setting,
            judge: value.judge,
            guide_se: value.guide_se,
            guide_se_vol: value.guide_se_vol,
            lift: value.lift,
            lift_rate: value.lift_rate,
            judge_ad: value.judge_ad,
            roof: value.roof,
            long_pop: value.long_pop,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "config")]
pub(super) struct ConfigData {
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

impl From<ConfigState> for ConfigData {
    fn from(value: ConfigState) -> Self {
        Self {
            mode: value.mode,
            chara: value.chara,
            music: value.music,
            sheet: value.sheet,
            category: value.category,
            sub_category: value.sub_category,
            chara_category: value.chara_category,
            ms_banner_disp: value.ms_banner_disp,
            ms_down_info: value.ms_down_info,
            ms_raise_type: value.ms_raise_type,
            banner_sort: value.banner_sort,
            disp_setting: value.disp_setting,
            h_vol: value.h_vol,
            hiscore_disp: value.hiscore_disp,
            lane_type: value.lane_type,
            brightness: value.brightness,
            key_beam: value.key_beam,
            lane_line: value.lane_line,
        }
    }
}

impl From<ConfigData> for ConfigState {
    fn from(value: ConfigData) -> Self {
        Self {
            mode: value.mode,
            chara: value.chara,
            music: value.music,
            sheet: value.sheet,
            category: value.category,
            sub_category: value.sub_category,
            chara_category: value.chara_category,
            ms_banner_disp: value.ms_banner_disp,
            ms_down_info: value.ms_down_info,
            ms_raise_type: value.ms_raise_type,
            banner_sort: value.banner_sort,
            disp_setting: value.disp_setting,
            h_vol: value.h_vol,
            hiscore_disp: value.hiscore_disp,
            lane_type: value.lane_type,
            brightness: value.brightness,
            key_beam: value.key_beam,
            lane_line: value.lane_line,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "item")]
pub(super) struct ItemData {
    #[kbin(rename = "type")]
    pub item_type: u8,
    pub id: u16,
    pub param: u16,
    pub is_new: bool,
    pub get_time: u64,
}

impl From<ItemState> for ItemData {
    fn from(value: ItemState) -> Self {
        Self {
            item_type: value.item_type,
            id: value.id,
            param: value.param,
            is_new: value.is_new,
            get_time: value.get_time,
        }
    }
}

impl From<ItemData> for ItemState {
    fn from(value: ItemData) -> Self {
        Self {
            item_type: value.item_type,
            id: value.id,
            param: value.param,
            is_new: value.is_new,
            get_time: value.get_time,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "chara_param")]
pub(super) struct CharacterData {
    pub chara_id: u16,
    pub friendship: u16,
}

impl From<CharacterState> for CharacterData {
    fn from(value: CharacterState) -> Self {
        Self {
            chara_id: value.chara_id,
            friendship: value.friendship,
        }
    }
}

impl From<CharacterData> for CharacterState {
    fn from(value: CharacterData) -> Self {
        Self {
            chara_id: value.chara_id,
            friendship: value.friendship,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "ex_info")]
pub(super) struct ExtraData {
    pub music_num: i16,
    pub point: u8,
    #[kbin(array)]
    pub ex_gauge_lv: Vec<u8>,
    #[kbin(array)]
    pub clear_lv: Vec<u8>,
}

impl From<ExtraState> for ExtraData {
    fn from(value: ExtraState) -> Self {
        Self {
            music_num: value.music_num,
            point: value.point,
            ex_gauge_lv: normalize(value.ex_gauge_lv, EXTRA_LEVEL_COUNT),
            clear_lv: normalize(value.clear_lv, EXTRA_LEVEL_COUNT),
        }
    }
}

impl From<ExtraData> for ExtraState {
    fn from(value: ExtraData) -> Self {
        Self {
            music_num: value.music_num,
            point: value.point,
            ex_gauge_lv: normalize(value.ex_gauge_lv, EXTRA_LEVEL_COUNT),
            clear_lv: normalize(value.clear_lv, EXTRA_LEVEL_COUNT),
        }
    }
}

fn normalize<T: Clone + Default>(mut values: Vec<T>, length: usize) -> Vec<T> {
    values.truncate(length);
    values.resize(length, T::default());
    values
}

#[derive(Kbin)]
#[kbin(node = "netvs")]
pub(super) struct NetvsWrite {
    #[kbin(array)]
    pub record: Vec<i16>,
    pub ojama_condition: String,
    #[kbin(array)]
    pub set_ojama: Vec<i8>,
    #[kbin(array)]
    pub set_recommend: Vec<i8>,
    #[kbin(repeated)]
    pub rival_id: Vec<String>,
    pub select_num_normal: u32,
    pub select_num_ojama: u32,
    pub netvs_play_cnt: u32,
}

impl From<NetvsWrite> for NetvsState {
    fn from(value: NetvsWrite) -> Self {
        Self {
            record: normalize(value.record, NETVS_RECORD_COUNT),
            dialogs: Vec::new(),
            ojama_condition: parse_condition(&value.ojama_condition),
            set_ojama: normalize_i8(value.set_ojama, NETVS_SET_COUNT),
            set_recommend: normalize_i8(value.set_recommend, NETVS_SET_COUNT),
            rival_ids: value.rival_id,
            select_num_normal: value.select_num_normal,
            select_num_ojama: value.select_num_ojama,
            netvs_play_cnt: value.netvs_play_cnt,
        }
    }
}

fn normalize_i8(mut values: Vec<i8>, length: usize) -> Vec<i8> {
    values.truncate(length);
    values.resize(length, 0);
    values
}

fn parse_condition(value: &str) -> Vec<i8> {
    let values = value
        .split(',')
        .filter_map(|part| part.trim().parse::<i8>().ok())
        .collect();
    normalize_i8(values, NETVS_OJAMA_CONDITION_COUNT)
}
