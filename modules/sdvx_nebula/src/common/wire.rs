use super::*;

#[derive(Kbin)]
#[kbin(node = "game")]
pub(crate) struct EmptyRequest {}

#[derive(Kbin)]
#[kbin(node = "game")]
struct CommonResponse {
    music_limited: Option<MusicLimited>,
    catalog: Option<Catalog>,
    event: Option<Event>,
    skill_course: Option<SkillCourse>,
    music: Option<MusicCatalog>,
    appealcard: Option<AppealCards>,
    akaname: Option<Akaname>,
    extend: Option<Extend>,
    automation: Option<AutomationCatalog>,
    festival: Option<Festival>,
    valgene: Option<Valgene>,
    arena: Option<Arena>,
    apigene: Option<Apigene>,
    volte_factory: Option<VolteFactory>,
    campaign: Option<Campaign>,
    something: Option<RankingEvent>,
    #[kbin(repeated)]
    weekly_music: Vec<WeeklyMusic>,
    invest: Option<Invest>,
}

impl CommonResponse {
    fn new(data: &CommonData) -> Self {
        Self {
            music_limited: (!data.music_limited.is_empty()).then(|| MusicLimited {
                info: data.music_limited.clone(),
            }),
            catalog: (!data.catalog.is_empty()).then(|| Catalog {
                info: data.catalog.clone(),
            }),
            event: (!data.events.is_empty()).then(|| Event {
                info: data.events.clone(),
            }),
            skill_course: (!data.skill_courses.is_empty()).then(|| SkillCourse {
                info: data.skill_courses.clone(),
            }),
            // These catalogs ship in the update's local XML databases. Common
            // carries only server-side deltas and unlock state; the codec can
            // still represent the complete catalog when a caller needs it.
            music: None,
            appealcard: None,
            akaname: None,
            extend: None,
            automation: None,
            festival: None,
            valgene: None,
            arena: None,
            apigene: None,
            volte_factory: None,
            campaign: None,
            something: None,
            weekly_music: Vec::new(),
            invest: None,
        }
    }
}

macro_rules! repeated_container {
    ($name:ident, $node:literal, $item:ty) => {
        #[derive(Kbin)]
        #[kbin(node = $node)]
        struct $name {
            #[kbin(repeated)]
            info: Vec<$item>,
        }
    };
}

repeated_container!(MusicLimited, "music_limited", MusicLimitedInfo);
repeated_container!(Catalog, "catalog", CatalogInfo);
repeated_container!(Event, "event", EventInfo);
repeated_container!(SkillCourse, "skill_course", SkillCourseInfo);
repeated_container!(MusicCatalog, "music", MusicInfo);
repeated_container!(AppealCards, "appealcard", AppealCardInfo);
repeated_container!(Akaname, "akaname", AkanamePart);
repeated_container!(Extend, "extend", ExtendInfo);
repeated_container!(AutomationCatalog, "automation", AutomationInfo);
repeated_container!(RankingEvent, "something", RankingEventInfo);

#[derive(Kbin)]
#[kbin(node = "campaign")]
struct Campaign {
    stock: Option<CampaignStock>,
}

#[derive(Kbin)]
#[kbin(node = "stock")]
struct CampaignStock {
    campaign_id: i32,
    stock_num: i32,
}

#[derive(Kbin)]
#[kbin(node = "festival")]
struct Festival {
    info: Option<FestivalInfo>,
    #[kbin(repeated)]
    catalog: Vec<FestivalCatalog>,
    #[kbin(repeated)]
    mission: Vec<FestivalMission>,
}

#[derive(Kbin)]
#[kbin(node = "valgene")]
struct Valgene {
    #[kbin(repeated)]
    info: Vec<ValgeneInfo>,
    #[kbin(repeated)]
    catalog: Vec<ValgeneCatalog>,
}

#[derive(Kbin)]
#[kbin(node = "arena")]
struct Arena {
    info: Option<ArenaInfo>,
    #[kbin(repeated)]
    catalog: Vec<FestivalCatalog>,
    #[kbin(repeated)]
    mission: Vec<FestivalMission>,
}

#[derive(Kbin)]
#[kbin(node = "apigene")]
struct Apigene {
    #[kbin(repeated)]
    info: Vec<ApigeneInfo>,
    #[kbin(repeated)]
    catalog: Vec<ApigeneCatalog>,
}

#[derive(Kbin)]
#[kbin(node = "volte_factory")]
struct VolteFactory {
    goods: Option<FactoryGoods>,
    stock: Option<FactoryStock>,
}

#[derive(Kbin)]
#[kbin(node = "goods")]
struct FactoryGoods {
    #[kbin(repeated)]
    info: Vec<FactoryGoodsInfo>,
}
#[derive(Kbin)]
#[kbin(node = "stock")]
struct FactoryStock {
    #[kbin(repeated)]
    info: Vec<FactoryStockInfo>,
}

#[derive(Kbin)]
#[kbin(node = "weekly_music")]
struct WeeklyMusic {
    week_id: i32,
    music_id: i32,
    time_start: u64,
    time_end: u64,
}
#[derive(Kbin)]
#[kbin(node = "invest")]
struct Invest {
    limit_date: u64,
}

#[derive(Kbin)]
#[kbin(node = "info")]
struct ExtendInfo {
    extend_type: u32,
    extend_id: u32,
    param_num_1: i32,
    param_num_2: i32,
    param_num_3: i32,
    param_num_4: i32,
    param_num_5: i32,
    param_str_1: Option<String>,
    param_str_2: Option<String>,
    param_str_3: Option<String>,
    param_str_4: Option<String>,
    param_str_5: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "info")]
struct AutomationInfo {
    mix_id: i32,
    mix_code: String,
    mix_name: String,
    seq: String,
    player_name: String,
    generate_param: String,
    distribution_date: u32,
    jacket_id: i32,
    tag_bit: i32,
    like_flg: bool,
}

#[derive(Kbin)]
#[kbin(node = "info")]
struct FestivalInfo {
    fes_id: i32,
    fes_name: String,
    time_start: u64,
    time_end: u64,
    shop_start: u64,
    shop_end: u64,
    is_open: bool,
    is_shop: bool,
}
#[derive(Kbin)]
#[kbin(node = "catalog")]
struct FestivalCatalog {
    catalog_id: i32,
    catalog_type: i32,
    price: i32,
    item_type: i32,
    item_id: i32,
    param: i32,
    num: i32,
}
#[derive(Kbin)]
#[kbin(node = "mission")]
struct FestivalMission {
    no: i32,
    mission_id: i32,
    mission_type: i32,
    param: i32,
    live_energy: i32,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct ValgeneInfo {
    valgene_name: String,
    name_english: String,
    valgene_id: i32,
    rarity: i32,
}
#[derive(Kbin)]
#[kbin(node = "catalog")]
struct ValgeneCatalog {
    item_type: i32,
    item_id: i32,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct ArenaInfo {
    season: i32,
    rule: i32,
    #[kbin(array)]
    rank_match_target: Vec<i32>,
    time_start: u64,
    time_end: u64,
    shop_start: u64,
    shop_end: u64,
    is_open: bool,
    is_shop: bool,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct ApigeneInfo {
    apigene_id: i32,
    name: String,
    name_english: String,
    common_rate: i32,
    uncommon_rate: i32,
    rare_rate: i32,
    price: i32,
    no_duplicate: bool,
}
#[derive(Kbin)]
#[kbin(node = "catalog")]
struct ApigeneCatalog {
    item_type: i32,
    item_id: i32,
    rarity: i32,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct FactoryGoodsInfo {
    factory_id: u8,
    goods_id: i32,
    goods_type: u8,
    name: String,
    tex_name: String,
    stock_num: i32,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct FactoryStockInfo {
    factory_id: u8,
    goods_id: i32,
    status: i32,
    get_refid: String,
    get_cardnumber: String,
    get_date: u64,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct RankingEventInfo {
    ranking_id: i32,
    ranking_type: i32,
    ranking_title: String,
    border_rank: i32,
    unit_str: String,
    time_start: u64,
    time_end: u64,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "info")]
pub(crate) struct MusicLimitedInfo {
    pub music_id: i32,
    pub music_type: u8,
    pub limited: u8,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "info")]
pub(crate) struct CatalogInfo {
    pub catalog_type: u8,
    pub catalog_id: u32,
    pub discount_rate: u32,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "info")]
pub(crate) struct EventInfo {
    pub event_id: String,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "info")]
pub(crate) struct SkillCourseInfo {
    pub season_id: i32,
    pub season_name: String,
    pub season_new_flg: bool,
    pub course_id: i16,
    pub course_name: String,
    pub course_type: i16,
    pub skill_level: i16,
    pub skill_name_id: i16,
    pub matching_assist: bool,
    pub clear_rate: i32,
    pub avg_score: u32,
    pub skill_type: i16,
    pub gauge_type: Option<i16>,
    pub paseli_type: Option<i16>,
    #[kbin(repeated)]
    pub track: Vec<CourseTrack>,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "track")]
pub(crate) struct CourseTrack {
    pub track_no: i16,
    pub music_id: i32,
    pub music_type: i8,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "info")]
pub(crate) struct MusicInfo {
    pub music_id: u32,
    pub title_name: String,
    pub title_yomigana: String,
    pub artist_name: String,
    pub artist_yomigana: String,
    pub license_text: String,
    pub ascii: String,
    pub bpm_max: u32,
    pub bpm_min: u32,
    pub date: u32,
    pub volume: u16,
    pub version: u32,
    pub inf_ver: u32,
    pub bg_no: u32,
    pub genre: u32,
    pub demo_pri: u32,
    #[kbin(rename = "NOVICE")]
    pub novice: Option<ChartInfo>,
    #[kbin(rename = "ADVANCED")]
    pub advanced: Option<ChartInfo>,
    #[kbin(rename = "EXHAUST")]
    pub exhaust: Option<ChartInfo>,
    #[kbin(rename = "INFINITE")]
    pub infinite: Option<ChartInfo>,
    #[kbin(rename = "MAXIMUM")]
    pub maximum: Option<ChartInfo>,
    #[kbin(rename = "ULTIMATE")]
    pub ultimate: Option<ChartInfo>,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "NOVICE")]
pub(crate) struct ChartInfo {
    pub illustrator: String,
    pub effected_by: String,
    pub level: u32,
    pub price: u32,
    pub limited: u32,
    pub jacket_print: u32,
    pub jacket_mask: u32,
    pub max_exscore: u32,
    pub radar: Radar,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "radar")]
pub(crate) struct Radar {
    pub notes: u32,
    pub peak: u32,
    pub tsumami: u32,
    pub tricky: u32,
    pub hand_trip: u32,
    pub one_hand: u32,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "info")]
pub(crate) struct AppealCardInfo {
    pub appeal_id: u16,
    pub texture: String,
    pub title: String,
    pub illustrator: String,
    pub message_a: String,
    pub message_b: String,
    pub message_c: String,
    pub message_d: String,
    pub message_e: String,
    pub message_f: String,
    pub message_g: String,
    pub message_h: String,
    pub date: u32,
    pub rarity: u8,
    pub generator_no: u8,
    pub is_default: u8,
    pub sort_no: u16,
    pub genre: u8,
    pub limited: u8,
}

#[derive(Clone, Debug, Deserialize, Kbin)]
#[kbin(node = "info")]
pub(crate) struct AkanamePart {
    pub part_id: i32,
    pub word: String,
    pub is_default: bool,
}

mod methods;
