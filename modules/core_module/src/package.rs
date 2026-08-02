use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use crate::{State, fault};

#[derive(Kbin)]
#[kbin(node = "package")]
struct Request {
    #[kbin(attr)]
    pkgtype: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "package")]
struct Response {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
    #[kbin(attr)]
    secondary: u8,
}

#[rpc("package.list")]
async fn list(ctx: RpcContext<State>, _request: Request) -> RpcResult<Response> {
    Ok(Response {
        expire: 900,
        fault: fault(&ctx.model),
        secondary: 0,
    })
}
