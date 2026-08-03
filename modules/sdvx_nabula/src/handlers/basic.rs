use std::net::Ipv4Addr;

use bson::{DateTime, doc};
use vibea3::{Ip4, Kbin, RpcContext, RpcResult, rpc};

use crate::{
    State,
    common::EmptyRequest,
    database::PlayRecord,
    protocol::{DEFAULT_LOUNGE_INTERVAL_SECONDS, DEFAULT_SHOP_NEXT_TIME_SECONDS},
};

use super::{database_error, invalid_session};

#[derive(Kbin)]
#[kbin(node = "game")]
struct EmptyResponse {}

#[derive(Kbin)]
#[kbin(node = "game")]
struct FrozenResponse {
    result: u8,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct LogRequest {
    loc_id: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct ExceptionRequest {
    lid: String,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct EntryEndRequest {
    eid: Option<u32>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct EntryStartRequest {
    c_ver: Option<u8>,
    p_num: Option<u8>,
    p_rest: Option<u8>,
    filter: Option<u8>,
    mid: Option<u32>,
    sec: Option<u32>,
    port: Option<u16>,
    gip: Option<Ip4>,
    lip: Option<Ip4>,
    claim: Option<u8>,
    entry_id: Option<u32>,
}

#[derive(Kbin)]
#[kbin(node = "entry")]
struct EntryAddress {
    port: u16,
    gip: Ip4,
    lip: Ip4,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct EntryStartResponse {
    entry_id: u32,
    #[kbin(repeated)]
    entry: Vec<EntryAddress>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct FrozenRequest {
    refid: String,
    sec: Option<u32>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct HiscoreRequest {
    locid: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct HiscoreResponse {
    sc: HiscoreScores,
}

#[derive(Kbin)]
#[kbin(node = "sc")]
struct HiscoreScores {
    #[kbin(repeated)]
    d: Vec<HiscoreEntry>,
}

#[derive(Kbin)]
#[kbin(node = "d")]
struct HiscoreEntry {
    id: u32,
    ty: u32,
    a_sq: String,
    a_nm: String,
    a_sc: u32,
    l_sq: String,
    l_nm: String,
    l_sc: u32,
    ax_sq: String,
    ax_nm: String,
    ax_sc: u32,
    lx_sq: String,
    lx_nm: String,
    lx_sc: u32,
    cr: i32,
    avg_sc: u32,
    avg_ex: u32,
    #[kbin(array)]
    avg_sc_lv: Vec<u32>,
    #[kbin(array)]
    avg_ex_lv: Vec<u32>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct LoungeRequest {
    filter: Option<u8>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct LoungeResponse {
    interval: u32,
    #[kbin(repeated)]
    wait: Vec<LoungeWait>,
}

#[derive(Kbin)]
#[kbin(node = "wait")]
struct LoungeWait {
    m_id: u32,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct PlayStartResponse {
    play_id: u32,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct SampleResponse {
    #[kbin(attr)]
    release: String,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct SampleRequest {
    ref_id: String,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct PlayEndRequest {
    refid: String,
    play_id: Option<u32>,
    start_type: Option<i8>,
    mode: Option<i8>,
    track_num: Option<i16>,
    s_coin: Option<i32>,
    s_paseli: Option<i32>,
    print_card: Option<u32>,
    print_result: Option<u32>,
    valgene_cnt: Option<u32>,
    blaster_num: Option<u32>,
    today_cnt: Option<u32>,
    play_chain: Option<u32>,
    week_play_cnt: Option<u32>,
    week_chain: Option<u32>,
    locid: Option<String>,
    drop_frame: Option<u16>,
    drop_frame_max: Option<u16>,
    drop_count: Option<u16>,
    etc: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct SerialRequest {
    refid: String,
    code: String,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct SerialResponse {
    serial_name: String,
    #[kbin(repeated)]
    item: Vec<SerialItem>,
    gamecoin_packet: u32,
    gamecoin_block: u32,
    result: i8,
}

#[derive(Kbin)]
#[kbin(node = "item")]
struct SerialItem {
    #[kbin(rename = "type")]
    item_type: u32,
    id: u32,
    param: u32,
    param_after: u32,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct ShopRequest {
    etc: Option<String>,
    setting: Option<ShopSetting>,
}

#[derive(Kbin)]
#[kbin(node = "setting")]
struct ShopSetting {
    coin_slot: Option<i32>,
    game_start: Option<i32>,
    schedule: Option<String>,
    reference: Option<String>,
    basic_rate: Option<String>,
    tax_rate: Option<i32>,
    time_service: Option<String>,
    service_value: Option<String>,
    service_limit: Option<String>,
    service_time: Option<String>,
    free_play: Option<i32>,
    free_first_play: Option<i32>,
    start_credits: Option<String>,
    valkyrie_credit: Option<i32>,
    button_count: Option<ButtonCount>,
}

#[derive(Kbin)]
#[kbin(node = "button_count")]
struct ButtonCount {
    start: Option<u32>,
    bt_a: Option<u32>,
    bt_b: Option<u32>,
    bt_c: Option<u32>,
    bt_d: Option<u32>,
    fx_l: Option<u32>,
    fx_r: Option<u32>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct ShopResponse {
    nxt_time: u32,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct UstaLinkRequest {
    usta_link: UstaLink,
}

#[derive(Kbin)]
#[kbin(node = "usta_link")]
struct UstaLink {
    ref_id: String,
    pin: i32,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct UstaLinkResponse {
    usta_link: UstaLinkResult,
}

#[derive(Kbin)]
#[kbin(node = "usta_link")]
struct UstaLinkResult {
    link_id: String,
}

mod methods;
