use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use crate::{State, fault};

#[derive(Kbin)]
#[kbin(node = "dlstatus")]
struct Request {}

#[derive(Kbin)]
#[kbin(node = "dlstatus")]
struct Response {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
}

#[rpc("dlstatus.progress")]
async fn progress(ctx: RpcContext<State>, _request: Request) -> RpcResult<Response> {
    Ok(Response {
        expire: 0,
        fault: fault(&ctx.model),
    })
}
