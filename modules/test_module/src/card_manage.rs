use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use crate::State;
use crate::pcb_tracker::{Request as AliveRequest, Response as AliveResponse};

#[derive(Kbin)]
#[kbin(node = "cardmng")]
pub(crate) struct Request {}

#[derive(Kbin)]
#[kbin(node = "cardmng")]
pub(crate) struct Response {
    value: u32,
}

#[rpc("cardmng.inquire")]
async fn inquire(ctx: RpcContext<State>, _request: Request) -> RpcResult<Response> {
    Ok(Response {
        value: ctx.state.value,
    })
}

#[derive(Kbin)]
#[kbin(node = "cardmng")]
struct ProbeResponse {
    alive: bool,
}

#[rpc("cardmng.probe")]
async fn probe(ctx: RpcContext<State>, _request: Request) -> RpcResult<ProbeResponse> {
    let response: AliveResponse = ctx.call("pcbtracker.alive", AliveRequest {}).await?;
    Ok(ProbeResponse {
        alive: response.alive,
    })
}

#[rpc("cardmng.slow")]
async fn slow(ctx: RpcContext<State>, _request: Request) -> RpcResult<Response> {
    futures_timer::Delay::new(std::time::Duration::from_millis(300)).await;
    Ok(Response {
        value: ctx.state.value,
    })
}

#[rpc("cardmng.panic")]
async fn panic_handler(_ctx: RpcContext<State>, _request: Request) -> RpcResult<Response> {
    panic!("test module panic")
}

#[rpc("cardmng.recurse")]
async fn recurse(ctx: RpcContext<State>, _request: Request) -> RpcResult<Response> {
    ctx.call("cardmng.recurse", Request {}).await
}
