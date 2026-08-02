use vibea3::Kbin;

use crate::database::{MusicOption, MusicScore};

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct ReadScoreRequest {
    pub ref_id: String,
    pub data_id: String,
    pub pref: i8,
    pub no: i8,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct ReadScoreResponse {
    pub con_flg: bool,
    #[kbin(repeated)]
    pub music: Vec<MusicScoreResponse>,
}

#[derive(Kbin)]
#[kbin(node = "music")]
pub(super) struct MusicScoreResponse {
    music_num: i16,
    sheet_num: u8,
    score: i32,
    clear_type: u8,
    clear_rank: u8,
    cnt: i16,
    score_ver: i32,
    clear_type_ver: u8,
}

impl From<MusicScore> for MusicScoreResponse {
    fn from(value: MusicScore) -> Self {
        Self {
            music_num: value.music_num,
            sheet_num: value.sheet_num,
            score: value.score,
            clear_type: value.clear_type,
            clear_rank: value.clear_rank,
            cnt: value.cnt,
            score_ver: value.score_ver,
            clear_type_ver: value.clear_type_ver,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct ReadOptionRequest {
    pub ref_id: String,
    pub data_id: String,
    pub music_num: i16,
    pub sheet_num: u8,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct ReadOptionResponse {
    pub option: Option<OptionResponse>,
}

#[derive(Kbin)]
#[kbin(node = "option")]
pub(super) struct OptionResponse {
    hispeed: i16,
    popkun: u8,
    hidden: bool,
    hidden_rate: i16,
    sudden: bool,
    sudden_rate: i16,
    randmir: i8,
    gauge_type: i8,
    ojama_0: u8,
    ojama_1: u8,
    forever_0: bool,
    forever_1: bool,
    full_setting: bool,
    judge: u8,
    guide_se: i8,
    guide_se_vol: u8,
    lift: bool,
    lift_rate: i16,
    judge_ad: i8,
    roof: i16,
    long_pop: i8,
}

impl From<MusicOption> for OptionResponse {
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

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct WriteMusicRequest {
    pub ref_id: String,
    pub data_id: String,
    pub name: String,
    pub chara_num: i16,
    pub mode: u8,
    pub play_id: i32,
    pub stage: u8,
    pub music_num: i16,
    pub sheet_num: u8,
    pub clear_type: u8,
    pub clear_rank: u8,
    pub score: i32,
    pub cool: i16,
    pub great: i16,
    pub good: i16,
    pub bad: i16,
    pub combo: i16,
    pub highlight: i16,
    pub gauge: i16,
    pub gauge_type: i8,
    pub is_netvs: i8,
    pub is_win: i8,
    pub is_image_store: bool,
    pub is_option_save: bool,
    pub hispeed: i16,
    pub popkun: u8,
    pub hidden: bool,
    pub hidden_rate: i16,
    pub sudden: bool,
    pub sudden_rate: i16,
    pub randmir: i8,
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
    pub slow: i16,
    pub fast: i16,
    pub ex_gauge_lv: i8,
    pub is_super_extra: i8,
    pub netvs_ojama_type: i8,
    pub netvs_type: i8,
    pub netvs_rank: i8,
    pub netvs_ojama_0: u8,
    pub netvs_ojama_1: u8,
    pub netvs_ojama_2: u8,
    pub play_date: u64,
    pub select_total_time: u16,
    pub select_used_time: u16,
    pub select_remain_time: u16,
    pub category: i8,
    pub sub_category: i8,
    #[kbin(array)]
    pub my_graph: Option<Vec<u8>>,
}

impl WriteMusicRequest {
    pub(super) fn ref_id(&self) -> String {
        self.ref_id.clone()
    }

    pub(super) fn data_id(&self) -> String {
        self.data_id.clone()
    }

    pub(super) fn option(&self) -> Option<MusicOption> {
        self.is_option_save.then_some(MusicOption {
            hispeed: self.hispeed,
            popkun: self.popkun,
            hidden: self.hidden,
            hidden_rate: self.hidden_rate,
            sudden: self.sudden,
            sudden_rate: self.sudden_rate,
            randmir: self.randmir,
            gauge_type: self.gauge_type,
            ojama_0: self.ojama_0,
            ojama_1: self.ojama_1,
            forever_0: self.forever_0,
            forever_1: self.forever_1,
            full_setting: self.full_setting,
            judge: self.judge,
            guide_se: self.guide_se,
            guide_se_vol: self.guide_se_vol,
            lift: self.lift,
            lift_rate: self.lift_rate,
            judge_ad: self.judge_ad,
            roof: self.roof,
            long_pop: self.long_pop,
        })
    }
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct BuyRequest {
    pub play_id: i32,
    pub ref_id: String,
    pub id: u16,
    #[kbin(rename = "type")]
    pub item_type: u8,
    pub param: u16,
    pub lumina: i32,
    pub price: u16,
}
