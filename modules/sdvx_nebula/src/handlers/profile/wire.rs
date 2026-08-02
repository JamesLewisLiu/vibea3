use super::*;

#[derive(Kbin)]
#[kbin(node = "game")]
struct LoadResponse {
    result: u8,
    name: Option<String>,
    code: Option<String>,
    sdvx_id: Option<String>,
    appeal_id: Option<u16>,
    skill_level: Option<i16>,
    skill_name_id: Option<i16>,
    skill_type: Option<i16>,
    gamecoin_packet: Option<u32>,
    gamecoin_block: Option<u32>,
    blaster_energy: Option<u32>,
    blaster_count: Option<u32>,
    variant_gate: Option<VariantGate>,
    play_count: Option<u32>,
    day_count: Option<u32>,
    today_count: Option<u32>,
    play_chain: Option<u32>,
    max_play_chain: Option<u32>,
    week_count: Option<u32>,
    week_play_count: Option<u32>,
    week_chain: Option<u32>,
    max_week_chain: Option<u32>,
    creator_id: Option<u32>,
    eaappli: Option<EaAppli>,
    ea_shop: Option<EaShop>,
    kac_id: Option<String>,
    block_no: Option<i32>,
    support_team_id: Option<i32>,
    setting: Option<SettingResponse>,
    item: Option<ItemContainer>,
    item_cloud: Option<ItemCloudContainer>,
    present: Option<PresentContainer>,
    param: Option<ParamContainer>,
    story: Option<StoryContainer>,
    music: Option<MusicContainer>,
    volte_factory: Option<FactoryState>,
    #[kbin(repeated)]
    campaign: Vec<CampaignState>,
    cloud: Option<CloudState>,
    something: Option<RankingState>,
    festival: Option<FestivalState>,
    valgene_ticket: Option<ValgeneTicket>,
    arena: Option<ArenaState>,
    additional_info: Option<AdditionalInfo>,
    #[kbin(repeated)]
    weekly_music: Vec<WeeklyMusicState>,
    floorinfection: Option<EnergyEvent>,
    pb: Option<EnergyEvent>,
}

impl LoadResponse {
    fn missing() -> Self {
        Self {
            result: 1,
            name: None,
            code: None,
            sdvx_id: None,
            appeal_id: None,
            skill_level: None,
            skill_name_id: None,
            skill_type: None,
            gamecoin_packet: None,
            gamecoin_block: None,
            blaster_energy: None,
            blaster_count: None,
            variant_gate: None,
            play_count: None,
            day_count: None,
            today_count: None,
            play_chain: None,
            max_play_chain: None,
            week_count: None,
            week_play_count: None,
            week_chain: None,
            max_week_chain: None,
            creator_id: None,
            eaappli: None,
            ea_shop: None,
            kac_id: None,
            block_no: None,
            support_team_id: None,
            setting: None,
            item: None,
            item_cloud: None,
            present: None,
            param: None,
            story: None,
            music: None,
            volte_factory: None,
            campaign: Vec::new(),
            cloud: None,
            something: None,
            festival: None,
            valgene_ticket: None,
            arena: None,
            additional_info: None,
            weekly_music: Vec::new(),
            floorinfection: None,
            pb: None,
        }
    }

    fn found(profile: PlayerProfile, scores: Vec<MusicScore>) -> Self {
        let variant_gate = VariantGate::from_profile(&profile);
        Self {
            result: 0,
            name: Some(profile.name),
            code: Some(profile.code),
            sdvx_id: Some(profile.sdvx_id),
            appeal_id: Some(profile.appeal_id),
            skill_level: Some(profile.skill_level),
            skill_name_id: Some(profile.skill_name_id),
            skill_type: profile.skill_type,
            gamecoin_packet: Some(profile.gamecoin_packet),
            gamecoin_block: Some(profile.gamecoin_block),
            blaster_energy: Some(profile.blaster_energy),
            blaster_count: Some(profile.blaster_count),
            variant_gate: Some(variant_gate),
            play_count: Some(profile.play_count),
            day_count: Some(profile.day_count),
            today_count: Some(profile.today_count),
            play_chain: Some(profile.play_chain),
            max_play_chain: Some(profile.max_play_chain),
            week_count: Some(profile.week_count),
            week_play_count: Some(profile.week_play_count),
            week_chain: Some(profile.week_chain),
            max_week_chain: Some(profile.max_week_chain),
            creator_id: Some(profile.creator_id),
            eaappli: Some(EaAppli {
                relation: profile.eaappli_relation,
            }),
            ea_shop: Some(EaShop {
                blaster_pass_enable: profile.blaster_pass_enable,
                blaster_pass_limit_date: profile.blaster_pass_limit_date,
            }),
            kac_id: Some(profile.kac_id),
            block_no: Some(profile.block_no),
            support_team_id: Some(profile.support_team_id),
            setting: Some(profile.setting.into()),
            item: Some(ItemContainer {
                info: profile.items.into_iter().map(Into::into).collect(),
            }),
            item_cloud: None,
            present: None,
            param: Some(ParamContainer {
                info: profile.params.into_iter().map(Into::into).collect(),
            }),
            story: Some(StoryContainer {
                info: profile.stories.into_iter().map(Into::into).collect(),
            }),
            music: Some(MusicContainer {
                info: scores.into_iter().map(ScoreResponse::from).collect(),
            }),
            volte_factory: None,
            campaign: Vec::new(),
            cloud: None,
            something: None,
            festival: None,
            valgene_ticket: None,
            arena: None,
            additional_info: None,
            weekly_music: Vec::new(),
            floorinfection: None,
            pb: None,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "variant_gate")]
struct VariantGate {
    power: i32,
    element: VariantElements,
    #[kbin(array)]
    over_radar: Vec<i32>,
}

impl VariantGate {
    fn from_profile(profile: &PlayerProfile) -> Self {
        Self {
            power: profile.variant_power,
            element: profile.variant_elements.clone().into(),
            over_radar: profile.over_radar.clone(),
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "element")]
struct VariantElements {
    notes: i32,
    peak: i32,
    tsumami: i32,
    tricky: i32,
    onehand: i32,
    handtrip: i32,
}
impl From<RadarElements> for VariantElements {
    fn from(v: RadarElements) -> Self {
        Self {
            notes: v.notes,
            peak: v.peak,
            tsumami: v.tsumami,
            tricky: v.tricky,
            onehand: v.onehand,
            handtrip: v.handtrip,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "eaappli")]
struct EaAppli {
    relation: i8,
}

#[derive(Kbin)]
#[kbin(node = "ea_shop")]
struct EaShop {
    blaster_pass_enable: bool,
    blaster_pass_limit_date: u64,
}

mod extra;
use extra::*;

#[derive(Kbin)]
#[kbin(node = "setting")]
struct SettingResponse {
    hispeed: i32,
    lanespeed: u32,
    gauge_option: u8,
    ars_option: u8,
    notes_option: u8,
    early_late_disp: u8,
    draw_adjust: i32,
    eff_c_left: u8,
    eff_c_right: u8,
    #[kbin(rename = "last_music_id")]
    music_id: i32,
    #[kbin(rename = "last_music_type")]
    music_type: u8,
    sort_type: u8,
    narrow_down: u8,
    headphone: u8,
}
impl From<PlayerSetting> for SettingResponse {
    fn from(v: PlayerSetting) -> Self {
        Self {
            hispeed: v.hispeed,
            lanespeed: v.lanespeed,
            gauge_option: v.gauge_option,
            ars_option: v.ars_option,
            notes_option: v.notes_option,
            early_late_disp: v.early_late_disp,
            draw_adjust: v.draw_adjust,
            eff_c_left: v.eff_c_left,
            eff_c_right: v.eff_c_right,
            music_id: v.music_id,
            music_type: v.music_type,
            sort_type: v.sort_type,
            narrow_down: v.narrow_down,
            headphone: v.headphone,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "item")]
struct ItemContainer {
    #[kbin(repeated)]
    info: Vec<ItemResponse>,
}
#[derive(Kbin)]
#[kbin(node = "item_cloud")]
struct ItemCloudContainer {
    #[kbin(repeated)]
    info: Vec<ItemResponse>,
}
#[derive(Kbin)]
#[kbin(node = "present")]
struct PresentContainer {
    #[kbin(repeated)]
    info: Vec<ItemResponse>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct ItemResponse {
    #[kbin(rename = "type")]
    item_type: u8,
    id: u32,
    param: u32,
}
impl From<PlayerItem> for ItemResponse {
    fn from(v: PlayerItem) -> Self {
        Self {
            item_type: u8::try_from(v.item_type).unwrap_or(u8::MAX),
            id: v.id,
            param: v.param,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "param")]
struct ParamContainer {
    #[kbin(repeated)]
    info: Vec<ParamResponse>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct ParamResponse {
    #[kbin(rename = "type")]
    param_type: i32,
    id: i32,
    #[kbin(array)]
    param: Vec<i32>,
}
impl From<PlayerParam> for ParamResponse {
    fn from(mut v: PlayerParam) -> Self {
        v.values.resize(PARAMETER_VALUE_COUNT, 0);
        Self {
            param_type: v.param_type,
            id: v.id,
            param: v.values,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "story")]
struct StoryContainer {
    #[kbin(repeated)]
    info: Vec<StoryResponse>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct StoryResponse {
    story_id: i32,
    progress_id: i32,
    progress_param: i32,
    clear_cnt: i32,
    route_flg: u32,
}
impl From<StoryProgress> for StoryResponse {
    fn from(v: StoryProgress) -> Self {
        Self {
            story_id: v.story_id,
            progress_id: v.progress_id,
            progress_param: v.progress_param,
            clear_cnt: v.clear_cnt,
            route_flg: v.route_flg,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "music")]
struct MusicContainer {
    #[kbin(repeated)]
    info: Vec<ScoreResponse>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct ScoreResponse {
    #[kbin(array)]
    param: Vec<u32>,
}
impl From<MusicScore> for ScoreResponse {
    fn from(v: MusicScore) -> Self {
        let mut param = vec![0; MUSIC_RECORD_PARAM_COUNT];
        param[MUSIC_PARAM_MUSIC_ID] = v.music_id;
        param[MUSIC_PARAM_MUSIC_TYPE] = v.music_type;
        param[MUSIC_PARAM_PRIMARY_SCORE] = v.score;
        param[MUSIC_PARAM_PRIMARY_EXSCORE] = v.exscore;
        param[MUSIC_PARAM_PRIMARY_CLEAR_TYPE] = v.clear_type;
        param[MUSIC_PARAM_PRIMARY_GRADE] = v.score_grade;
        param[MUSIC_PARAM_PRIMARY_MAX_CHAIN] = v.max_chain;
        param[MUSIC_PARAM_PRIMARY_PLAY_COUNT] = v.play_count;
        param[MUSIC_PARAM_PRIMARY_CLEAR_COUNT] = v.clear_count;
        param[MUSIC_PARAM_PRIMARY_UC_COUNT] = v.ultimate_chain_count;
        param[MUSIC_PARAM_PRIMARY_PUC_COUNT] = v.perfect_ultimate_chain_count;
        param[MUSIC_PARAM_VOLFORCE] = v.volforce;
        param[MUSIC_PARAM_BUTTON_RATE] = v.btn_rate;
        param[MUSIC_PARAM_LONG_RATE] = v.long_rate;
        param[MUSIC_PARAM_VOL_RATE] = v.vol_rate;
        Self { param }
    }
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct RivalResponse {
    #[kbin(repeated)]
    rival: Vec<RivalInfo>,
    #[kbin(repeated)]
    music: Vec<RivalMusic>,
}
#[derive(Kbin)]
#[kbin(node = "rival")]
struct RivalInfo {
    no: i16,
    seq: String,
    name: String,
}
#[derive(Kbin)]
#[kbin(node = "music")]
struct RivalMusic {
    #[kbin(array)]
    param: Vec<u32>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct AutomationResponse {
    result: i32,
    automation: Option<Automation>,
}
#[derive(Kbin)]
#[kbin(node = "automation")]
struct Automation {
    mix_id: Option<i32>,
    mix_code: Option<String>,
    mix_name: Option<String>,
    seq: Option<String>,
    player_name: Option<String>,
    generate_param: Option<String>,
    distribution_date: Option<u32>,
    tag_bit: Option<i32>,
    jacket_id: Option<i32>,
    like_flg: Option<bool>,
    etc: Option<String>,
}

mod methods;

#[cfg(test)]
mod tests;
