use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use crate::State;

#[derive(Kbin)]
#[kbin(node = "loctest24")]
struct LocTestRequest {
    #[kbin(rename = "locTest")]
    loc_test: LocTest,
}

#[derive(Kbin)]
#[kbin(node = "locTest")]
struct LocTest {
    mode: u8,
    chara_num: i16,
    stage_num: u8,
    locid: String,
    #[kbin(repeated)]
    stage: Vec<LocTestStage>,
}

#[derive(Kbin)]
#[kbin(node = "stage")]
struct LocTestStage {
    music_num: i16,
    sheet_num: u8,
    clear_type: u8,
    clear_rank: u8,
    groove_gauge: i16,
    score: i32,
    cool: i16,
    great: i16,
    good: i16,
    bad: i16,
    highlight: i16,
    combo: i32,
    option_full: bool,
    hi_speed: i8,
    pop_kun: i8,
    gauge_type: i8,
    random: i8,
    hidden: bool,
    hidden_rate: i16,
    sudden: bool,
    sudden_rate: i16,
    ojama1: i8,
    ojama1_forever: bool,
    ojama2: i8,
    ojama2_forever: bool,
}

#[derive(Kbin)]
#[kbin(node = "loctest24")]
struct LocTestResponse {}

#[rpc("loctest24.log")]
async fn loctest(_ctx: RpcContext<State>, request: LocTestRequest) -> RpcResult<LocTestResponse> {
    let _ = request;
    Ok(LocTestResponse {})
}
