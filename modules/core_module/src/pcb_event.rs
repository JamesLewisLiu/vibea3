use vibea3::{Kbin, RpcContext, RpcResult, Timestamp, rpc};

use crate::{State, fault};

#[derive(Kbin)]
#[kbin(node = "pcbevent")]
struct Request {
    time: Option<Timestamp>,
    seq: Option<u32>,
    #[kbin(repeated)]
    item: Vec<Item>,
}

#[derive(Kbin)]
#[kbin(node = "item")]
struct Item {
    name: String,
    value: i32,
    time: Timestamp,
}

#[derive(Kbin)]
#[kbin(node = "pcbevent")]
struct Response {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
}

#[rpc("pcbevent.put")]
async fn put(ctx: RpcContext<State>, request: Request) -> RpcResult<Response> {
    let _ = request;
    Ok(Response {
        expire: 0,
        fault: fault(&ctx.model),
    })
}
