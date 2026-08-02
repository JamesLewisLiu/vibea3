use std::time::{SystemTime, UNIX_EPOCH};

use vibea3::{Kbin, RpcContext, RpcError, RpcResponse, RpcResult, rpc};

use crate::{State, fault};

#[derive(Kbin)]
#[kbin(node = "pcbtracker")]
struct Request {
    #[kbin(attr)]
    agree: Option<u8>,
    #[kbin(attr)]
    accountid: Option<String>,
    #[kbin(attr)]
    ecflag: Option<u8>,
    #[kbin(attr)]
    hardid: Option<String>,
    #[kbin(attr)]
    softid: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "pcbtracker")]
struct Response {
    #[kbin(attr)]
    alive: u8,
    #[kbin(attr)]
    ecenable: u8,
    #[kbin(attr)]
    eclimit: i32,
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
    #[kbin(attr)]
    limit: i32,
    #[kbin(attr)]
    time: u32,
}

#[rpc("pcbtracker.alive")]
async fn alive(ctx: RpcContext<State>, request: Request) -> RpcResult<RpcResponse<Response>> {
    let pcb_id = ctx.srcid.clone().unwrap_or_default();
    let _ = request;
    let machine = ctx
        .state
        .database
        .machine(pcb_id)
        .await
        .map_err(database_error)?;
    let prefix = ctx.model.split(':').next().unwrap_or_default();
    let eacoin_enabled = ctx.state.config.infinite_eacoin
        || machine
            .as_ref()
            .map(|value| value.eacoin_enabled)
            .unwrap_or(true);
    let ecenable = if eacoin_enabled {
        if prefix == "LMA" { 3 } else { 1 }
    } else {
        0
    };
    Ok(RpcResponse::new(
        0,
        Response {
            alive: 1,
            ecenable,
            eclimit: 0,
            expire: 600,
            fault: fault(&ctx.model),
            limit: 0,
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32,
        },
    ))
}

fn database_error(error: String) -> RpcError {
    RpcError::new(1, format!("database_error:{error}"))
}
