use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use crate::State;

#[derive(Kbin)]
#[kbin(node = "pcbtracker")]
pub(crate) struct Request {}

#[derive(Kbin)]
#[kbin(node = "pcbtracker")]
pub(crate) struct Response {
    pub(crate) alive: bool,
}

#[rpc("pcbtracker.alive")]
async fn alive(_ctx: RpcContext<State>, _request: Request) -> RpcResult<Response> {
    Ok(Response { alive: true })
}
