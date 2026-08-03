use super::*;

#[derive(Kbin)]
#[kbin(node = "volte_factory")]
pub(super) struct FactoryState {
    #[kbin(repeated)]
    info: Vec<FactoryStateInfo>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
pub(super) struct FactoryStateInfo {
    goods_id: i32,
    status: i32,
}
#[derive(Kbin)]
#[kbin(node = "campaign")]
pub(super) struct CampaignState {
    campaign_id: i32,
    jackpot_flg: bool,
}
#[derive(Kbin)]
#[kbin(node = "cloud")]
pub(super) struct CloudState {
    relation: i8,
}
#[derive(Kbin)]
#[kbin(node = "something")]
pub(super) struct RankingState {
    #[kbin(repeated)]
    info: Vec<RankingStateInfo>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
pub(super) struct RankingStateInfo {
    ranking_id: i32,
    value: i64,
}
#[derive(Kbin)]
#[kbin(node = "festival")]
pub(super) struct FestivalState {
    fes_id: i32,
    live_energy: i32,
    #[kbin(repeated)]
    bonus: Vec<FestivalBonus>,
}
#[derive(Kbin)]
#[kbin(node = "bonus")]
pub(super) struct FestivalBonus {
    energy_type: i32,
    live_energy: i32,
}
#[derive(Kbin)]
#[kbin(node = "valgene_ticket")]
pub(super) struct ValgeneTicket {
    ticket_num: i32,
    limit_date: u64,
}
#[derive(Kbin)]
#[kbin(node = "arena")]
pub(super) struct ArenaState {
    last_play_season: i32,
    rank_point: i32,
    shop_point: i32,
    ultimate_rate: i32,
    ultimate_rank_num: i32,
    rank_play_cnt: i32,
    ultimate_play_cnt: i32,
    megamix_rate: i32,
}
#[derive(Kbin)]
#[kbin(node = "additional_info")]
pub(super) struct AdditionalInfo {
    #[kbin(repeated)]
    info: Vec<AdditionalInfoEntry>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
pub(super) struct AdditionalInfoEntry {
    #[kbin(attr)]
    val: String,
}
#[derive(Kbin)]
#[kbin(node = "weekly_music")]
pub(super) struct WeeklyMusicState {
    week_id: i32,
    music_id: i32,
    music_type: i32,
    exscore: i32,
    rank: i32,
}
#[derive(Kbin)]
#[kbin(node = "floorinfection")]
pub(super) struct EnergyEvent {
    #[kbin(repeated)]
    info: Vec<EnergyEventInfo>,
    energy: Option<i32>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
pub(super) struct EnergyEventInfo {
    target_id: i32,
    exp: i32,
    start_date: u64,
    end_date: u64,
    #[kbin(repeated)]
    music: Vec<EnergyMusic>,
}
#[derive(Kbin)]
#[kbin(node = "music")]
pub(super) struct EnergyMusic {
    no: i32,
    point: i32,
    music_id: i32,
}
