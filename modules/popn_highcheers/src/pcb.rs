mod boot;
mod loctest;
#[cfg(test)]
mod tests;
mod write;

use bson::{DateTime, doc};
use vibea3::{Kbin, RpcContext, RpcError, RpcResult, rpc};

use crate::State;

#[derive(Kbin)]
#[kbin(node = "pcb")]
pub(super) struct EmptyResponse {}

#[derive(Kbin)]
#[kbin(node = "pcb")]
struct DlStatusRequest {
    lid: String,
    prg: i32,
}

#[rpc("pcb.dlstatus")]
async fn dlstatus(_ctx: RpcContext<State>, request: DlStatusRequest) -> RpcResult<EmptyResponse> {
    let _ = (request.lid, request.prg);
    Ok(EmptyResponse {})
}

#[derive(Kbin)]
#[kbin(node = "pcb")]
struct ErrorRequest {
    loc_id: String,
    code: String,
    scene: i8,
    info: String,
}

#[rpc("pcb.error")]
async fn error(ctx: RpcContext<State>, request: ErrorRequest) -> RpcResult<EmptyResponse> {
    ctx.state
        .database
        .record_error(doc! {
            "srcid": ctx.srcid.unwrap_or_default(),
            "loc_id": request.loc_id,
            "code": request.code,
            "scene": i32::from(request.scene),
            "info": request.info,
            "created_at": DateTime::now(),
        })
        .await
        .map_err(database_error)?;
    Ok(EmptyResponse {})
}

pub(super) fn database_error(error: String) -> RpcError {
    RpcError::new(1, format!("database_error:{error}"))
}
