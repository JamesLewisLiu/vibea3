use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use super::{EmptyResponse, boot::PcbStatus, database_error};
use crate::State;

#[derive(Kbin)]
#[kbin(node = "pcb")]
pub(super) struct WriteRequest {
    pcb_status: PcbStatus,
    pcb_setting: PcbSetting,
    #[kbin(repeated)]
    vc_setting: Vec<VideoSetting>,
    pcb_card: PcbCard,
    dlstatus: i32,
}

#[derive(Kbin)]
#[kbin(node = "pcb_setting")]
struct PcbSetting {
    loc_id: String,
    name: String,
    ds: i8,
    sa: i16,
    pmc: bool,
    m_st: i8,
    m_st_p_k: i8,
    c_enbl: bool,
    ch: i16,
    cm: i16,
    fr: bool,
    ffp: bool,
    ctc: i8,
    cts: i8,
    cts_p: i8,
    cts_p_k: i8,
    ec_enbl: bool,
    tr: i8,
    sv: i16,
    ecp_s: i16,
    ecp_c1: i16,
    ecp_sp: i16,
    ecp_cfp: i16,
    #[kbin(array)]
    schedule: Vec<i8>,
    bk_enbl: bool,
    set_c: bool,
    lc_sec: i16,
    rv_sec: i16,
    #[kbin(array)]
    sw_cnt: Vec<u32>,
    #[kbin(array)]
    c_enbl_w: Vec<bool>,
    #[kbin(array)]
    ch_w: Vec<i16>,
    #[kbin(array)]
    cm_w: Vec<i16>,
}

#[derive(Kbin)]
#[kbin(node = "vc_setting")]
struct VideoSetting {
    ptn_id: i16,
    br: i16,
    pmc: i16,
    cr: i16,
    s_enbl: bool,
    s_val: i16,
    s_lim: i16,
    bh: i16,
    bm: i16,
    eh: i16,
    em: i16,
}

#[derive(Kbin)]
#[kbin(node = "pcb_card")]
struct PcbCard {
    card_enable: i8,
    card_soldout: i8,
}

#[rpc("pcb.write")]
async fn write(ctx: RpcContext<State>, request: WriteRequest) -> RpcResult<EmptyResponse> {
    ctx.state
        .database
        .save_cabinet_name(
            ctx.srcid.as_deref().unwrap_or_default(),
            request.pcb_setting.name.clone(),
        )
        .await
        .map_err(database_error)?;
    let _ = (
        request.pcb_status,
        request.pcb_setting,
        request.vc_setting,
        request.pcb_card,
        request.dlstatus,
    );
    Ok(EmptyResponse {})
}
