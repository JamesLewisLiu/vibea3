use bson::DateTime;
use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use crate::{
    State,
    database::{
        MatchingResult, MusicScore, PlayerItem, PlayerParam, PlayerSetting, RadarElements,
        StoryProgress,
    },
    protocol::{
        CLEAR_TYPE_CLEAR, CLEAR_TYPE_PERFECT_ULTIMATE_CHAIN, CLEAR_TYPE_ULTIMATE_CHAIN,
        PARAMETER_VALUE_COUNT,
    },
};

use super::{database_error, invalid_session};

#[derive(Kbin)]
#[kbin(node = "game")]
struct EmptyResponse {}

#[derive(Kbin)]
#[kbin(node = "game")]
struct SaveRequest {
    play_id: Option<u32>,
    refid: String,
    locid: Option<String>,
    appeal_id: Option<u16>,
    skill_level: Option<i16>,
    skill_base_id: Option<i16>,
    skill_name_id: Option<i16>,
    skill_type: Option<i16>,
    earned_gamecoin_packet: Option<i32>,
    earned_gamecoin_block: Option<i32>,
    earned_blaster_energy: Option<i32>,
    variant_gate: Option<VariantEarned>,
    p_start: Option<u64>,
    p_end: Option<u64>,
    ea_shop: Option<EaShopWrite>,
    festival: Option<FestivalWrite>,
    arena: Option<ArenaWrite>,
    setting: Option<SettingWrite>,
    item: Option<ItemWrites>,
    param: Option<ParamWrites>,
    story: Option<StoryWrites>,
    #[kbin(repeated)]
    course: Vec<CourseWrite>,
    #[kbin(repeated)]
    track: Vec<TrackWrite>,
    print: Option<PrintWrite>,
}

#[derive(Kbin)]
#[kbin(node = "variant_gate")]
struct VariantEarned {
    earned_power: Option<i32>,
    earned_element: Option<ElementWrite>,
    #[kbin(array)]
    over_radar: Option<Vec<i32>>,
}
#[derive(Kbin)]
#[kbin(node = "earned_element")]
struct ElementWrite {
    notes: Option<i32>,
    peak: Option<i32>,
    tsumami: Option<i32>,
    tricky: Option<i32>,
    onehand: Option<i32>,
    handtrip: Option<i32>,
}
#[derive(Kbin)]
#[kbin(node = "ea_shop")]
struct EaShopWrite {
    used_packet_booster: Option<i32>,
    used_block_booster: Option<i32>,
}
#[derive(Kbin)]
#[kbin(node = "festival")]
struct FestivalWrite {
    fes_id: Option<i32>,
    earned_live_energy: Option<i32>,
    #[kbin(repeated)]
    history: Vec<FestivalHistory>,
}
#[derive(Kbin)]
#[kbin(node = "history")]
struct FestivalHistory {
    energy_type: Option<i32>,
    live_energy: Option<i32>,
}
#[derive(Kbin)]
#[kbin(node = "arena")]
struct ArenaWrite {
    season: Option<i32>,
    earned_rank_point: Option<i32>,
    earned_shop_point: Option<i32>,
    earned_ultimate_rate: Option<i32>,
    earned_megamix_rate: Option<i32>,
    rank_play: Option<bool>,
    ultimate_play: Option<bool>,
}

#[derive(Kbin)]
#[kbin(node = "setting")]
struct SettingWrite {
    hispeed: Option<i32>,
    lanespeed: Option<u32>,
    gauge_option: Option<u8>,
    ars_option: Option<u8>,
    notes_option: Option<u8>,
    early_late_disp: Option<u8>,
    draw_adjust: Option<i32>,
    eff_c_left: Option<u8>,
    eff_c_right: Option<u8>,
    music_id: Option<i32>,
    music_type: Option<u8>,
    sort_type: Option<u8>,
    narrow_down: Option<u8>,
    headphone: Option<u8>,
}

impl SettingWrite {
    fn apply(self, value: &mut PlayerSetting) {
        if let Some(v) = self.hispeed {
            value.hispeed = v
        }
        if let Some(v) = self.lanespeed {
            value.lanespeed = v
        }
        if let Some(v) = self.gauge_option {
            value.gauge_option = v
        }
        if let Some(v) = self.ars_option {
            value.ars_option = v
        }
        if let Some(v) = self.notes_option {
            value.notes_option = v
        }
        if let Some(v) = self.early_late_disp {
            value.early_late_disp = v
        }
        if let Some(v) = self.draw_adjust {
            value.draw_adjust = v
        }
        if let Some(v) = self.eff_c_left {
            value.eff_c_left = v
        }
        if let Some(v) = self.eff_c_right {
            value.eff_c_right = v
        }
        if let Some(v) = self.music_id {
            value.music_id = v
        }
        if let Some(v) = self.music_type {
            value.music_type = v
        }
        if let Some(v) = self.sort_type {
            value.sort_type = v
        }
        if let Some(v) = self.narrow_down {
            value.narrow_down = v
        }
        if let Some(v) = self.headphone {
            value.headphone = v
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "item")]
struct ItemWrites {
    #[kbin(repeated)]
    info: Vec<ItemWrite>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct ItemWrite {
    #[kbin(rename = "type")]
    item_type: u32,
    id: Option<u32>,
    param: Option<u32>,
    diff_param: Option<i32>,
}
#[derive(Kbin)]
#[kbin(node = "param")]
struct ParamWrites {
    #[kbin(repeated)]
    info: Vec<ParamWrite>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct ParamWrite {
    #[kbin(rename = "type")]
    param_type: i32,
    id: i32,
    #[kbin(array)]
    param: Vec<i32>,
}
#[derive(Kbin)]
#[kbin(node = "story")]
struct StoryWrites {
    #[kbin(repeated)]
    info: Vec<StoryWrite>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct StoryWrite {
    story_id: i32,
    progress_id: i32,
    progress_param: i32,
    clear_cnt: i32,
    route_flg: u32,
}

#[derive(Kbin)]
#[kbin(node = "course")]
struct CourseWrite {
    ssnid: Option<i16>,
    crsid: Option<i16>,
    st: Option<i16>,
    sc: Option<u32>,
    ex: Option<u32>,
    ct: Option<i16>,
    gr: Option<i16>,
    jr: Option<u32>,
    cr: Option<u32>,
    nr: Option<u32>,
    er: Option<u32>,
    cm: Option<u32>,
    ar: Option<i16>,
    #[kbin(repeated)]
    tr: Vec<CourseTrack>,
    kac_id: Option<String>,
}
#[derive(Kbin)]
#[kbin(node = "tr")]
struct CourseTrack {
    st: Option<i16>,
    sc: Option<u32>,
    ex: Option<u32>,
    ct: Option<u32>,
    gr: Option<u32>,
    jr: Option<u32>,
    cr: Option<u32>,
    nr: Option<u32>,
    er: Option<u32>,
    pr: Option<u32>,
}

#[derive(Clone, Kbin)]
#[kbin(node = "track")]
struct TrackWrite {
    play_id: Option<u32>,
    track_no: Option<u16>,
    music_id: u32,
    music_type: u32,
    score: u32,
    exscore: Option<u32>,
    volforce: Option<u32>,
    clear_type: Option<u32>,
    score_grade: Option<u32>,
    max_chain: Option<u32>,
    just: Option<u32>,
    critical: Option<u32>,
    near: Option<u32>,
    error: Option<u32>,
    effective_rate: Option<u32>,
    btn_rate: Option<u32>,
    long_rate: Option<u32>,
    vol_rate: Option<u32>,
    mode: Option<u8>,
    start_option: Option<u8>,
    gauge_type: Option<u8>,
    notes_option: Option<u8>,
    online_num: Option<u16>,
    local_num: Option<u16>,
    challenge_type: Option<u8>,
    retry_cnt: Option<i32>,
    #[kbin(array)]
    judge: Option<Vec<i32>>,
    drop_frame: Option<u16>,
    drop_frame_max: Option<u16>,
    drop_count: Option<u16>,
    etc: Option<String>,
    mix_id: Option<i32>,
    mix_like: Option<bool>,
    #[kbin(repeated)]
    matching: Vec<MatchingWrite>,
}
#[derive(Clone, Kbin)]
#[kbin(node = "matching")]
struct MatchingWrite {
    code: Option<String>,
    score: Option<u32>,
}
#[derive(Kbin)]
#[kbin(node = "print")]
struct PrintWrite {
    count: Option<i32>,
    start_option: Option<i8>,
}

impl TrackWrite {
    fn score(&self, user_id: &str) -> MusicScore {
        MusicScore {
            id: format!("{user_id}:{}:{}", self.music_id, self.music_type),
            user_id: user_id.to_owned(),
            music_id: self.music_id,
            music_type: self.music_type,
            score: self.score,
            exscore: self.exscore.unwrap_or_default(),
            clear_type: self.clear_type.unwrap_or_default(),
            score_grade: self.score_grade.unwrap_or_default(),
            max_chain: self.max_chain.unwrap_or_default(),
            best_critical: self.critical.unwrap_or_default(),
            best_near: self.near.unwrap_or_default(),
            best_error: self.error.unwrap_or_default(),
            volforce: self.volforce.unwrap_or_default(),
            just: self.just.unwrap_or_default(),
            effective_rate: self.effective_rate.unwrap_or_default(),
            btn_rate: self.btn_rate.unwrap_or_default(),
            long_rate: self.long_rate.unwrap_or_default(),
            vol_rate: self.vol_rate.unwrap_or_default(),
            mode: self.mode.unwrap_or_default(),
            start_option: self.start_option.unwrap_or_default(),
            gauge_type: self.gauge_type.unwrap_or_default(),
            notes_option: self.notes_option.unwrap_or_default(),
            online_num: self.online_num.unwrap_or_default(),
            local_num: self.local_num.unwrap_or_default(),
            challenge_type: self.challenge_type.unwrap_or_default(),
            retry_cnt: self.retry_cnt.unwrap_or_default(),
            judge: self.judge.clone().unwrap_or_default(),
            mix_id: self.mix_id.unwrap_or_default(),
            mix_like: self.mix_like.unwrap_or_default(),
            matching: self
                .matching
                .iter()
                .map(|entry| MatchingResult {
                    code: entry.code.clone().unwrap_or_default(),
                    score: entry.score.unwrap_or_default(),
                })
                .collect(),
            play_count: 1,
            clear_count: u32::from(self.clear_type.unwrap_or_default() >= CLEAR_TYPE_CLEAR),
            ultimate_chain_count: u32::from(
                self.clear_type.unwrap_or_default() >= CLEAR_TYPE_ULTIMATE_CHAIN,
            ),
            perfect_ultimate_chain_count: u32::from(
                self.clear_type.unwrap_or_default() >= CLEAR_TYPE_PERFECT_ULTIMATE_CHAIN,
            ),
            updated_at: DateTime::now(),
        }
    }
}

mod methods;

#[cfg(test)]
mod tests;
