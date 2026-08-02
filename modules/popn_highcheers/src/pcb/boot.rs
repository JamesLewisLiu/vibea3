use vibea3::{Ip4, Kbin, RpcContext, RpcResult, rpc};

use super::EmptyResponse;
use crate::State;

macro_rules! pcb_status {
    ($name:ident, $node:literal) => {
        #[derive(Kbin)]
        #[kbin(node = $node)]
        pub(super) struct $name {
            pub loc_id: String,
            pub loc_type: u8,
            pub loc_name: String,
            pub country: String,
            pub region: String,
            pub pref: i16,
            pub customer: String,
            pub company: String,
            pub gip: Ip4,
            pub gp: u16,
            pub rom_number: String,
            pub c_drive: u64,
            pub d_drive: u64,
            pub e_drive: u64,
            pub f_drive: u64,
            pub os_act: String,
            pub etc: String,
        }
    };
}

pcb_status!(BootRequest, "pcb");
pcb_status!(PcbStatus, "pcb_status");

#[rpc("pcb.boot")]
async fn boot(_ctx: RpcContext<State>, request: BootRequest) -> RpcResult<EmptyResponse> {
    let _ = request;
    Ok(EmptyResponse {})
}
