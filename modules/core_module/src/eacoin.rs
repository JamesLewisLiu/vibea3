use crate::{EACOIN_SESSION_ID_LENGTH, State, database::ConsumeInput, fault};
use vibea3::{Kbin, RpcContext, RpcError, RpcResponse, RpcResult, rpc};

const INFINITE_EACOIN_BALANCE: i32 = 9_999_999;

#[derive(Kbin)]
#[kbin(node = "eacoin")]
struct CheckInRequest {
    cardtype: Option<String>,
    cardid: String,
    passwd: Option<String>,
    ectype: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "eacoin")]
struct CheckInResponse {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
    sequence: Option<i16>,
    acstatus: Option<u8>,
    acid: Option<String>,
    acname: Option<String>,
    balance: Option<i32>,
    sessid: Option<String>,
    inshopcharge: Option<u8>,
    point: Option<i32>,
}

#[rpc("eacoin.checkin")]
async fn checkin(
    ctx: RpcContext<State>,
    request: CheckInRequest,
) -> RpcResult<RpcResponse<CheckInResponse>> {
    let balance = if !valid_card_id(&request.cardid) {
        None
    } else if ctx.state.config.infinite_eacoin {
        Some(INFINITE_EACOIN_BALANCE)
    } else {
        ctx.state
            .database
            .eacoin_balance(request.cardid.clone())
            .await
            .map_err(database_error)?
    };
    let Some(balance) = balance else {
        return Ok(RpcResponse::new(1, empty_checkin(&ctx.model)));
    };
    Ok(RpcResponse::new(
        0,
        CheckInResponse {
            expire: 0,
            fault: fault(&ctx.model),
            sequence: Some(0),
            acstatus: Some(0),
            acid: Some("acid".into()),
            acname: Some("acname".into()),
            balance: Some(balance),
            sessid: Some(request.cardid),
            inshopcharge: Some(0),
            point: Some(0),
        },
    ))
}

fn empty_checkin(model: &str) -> CheckInResponse {
    CheckInResponse {
        expire: 0,
        fault: fault(model),
        sequence: None,
        acstatus: None,
        acid: None,
        acname: None,
        balance: None,
        sessid: None,
        inshopcharge: None,
        point: None,
    }
}

#[derive(Kbin)]
#[kbin(node = "eacoin")]
struct CheckOutRequest {
    sessid: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "eacoin")]
struct CheckOutResponse {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
    sessid: String,
}

#[rpc("eacoin.checkout")]
async fn checkout(
    ctx: RpcContext<State>,
    _request: CheckOutRequest,
) -> RpcResult<CheckOutResponse> {
    Ok(CheckOutResponse {
        expire: 0,
        fault: fault(&ctx.model),
        sessid: "sessid".into(),
    })
}

#[derive(Kbin)]
#[kbin(node = "eacoin")]
struct ConsumeRequest {
    #[kbin(attr)]
    esdate: Option<String>,
    #[kbin(attr)]
    esid: Option<String>,
    sessid: String,
    sequence: Option<i16>,
    payment: i32,
    service: Option<i32>,
    itemtype: Option<String>,
    detail: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "eacoin")]
struct ConsumeResponse {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
    acstatus: Option<u8>,
    autocharge: Option<u8>,
    balance: Option<i32>,
}

#[rpc("eacoin.consume")]
async fn consume(
    ctx: RpcContext<State>,
    request: ConsumeRequest,
) -> RpcResult<RpcResponse<ConsumeResponse>> {
    if !valid_card_id(&request.sessid) || request.payment < 0 {
        return Ok(RpcResponse::new(1, empty_consume(&ctx.model)));
    }
    if ctx.state.config.infinite_eacoin {
        return Ok(RpcResponse::new(
            0,
            ConsumeResponse {
                expire: 0,
                fault: fault(&ctx.model),
                acstatus: Some(0),
                autocharge: Some(0),
                balance: Some(INFINITE_EACOIN_BALANCE),
            },
        ));
    }
    let result = ctx
        .state
        .database
        .consume_eacoin(ConsumeInput {
            card_id: request.sessid,
            pcb_id: ctx.srcid.clone().unwrap_or_default(),
            payment: request.payment,
            service: request.service.unwrap_or_default(),
            item_type: request.itemtype.unwrap_or_default(),
            detail: request.detail.unwrap_or_default(),
        })
        .await
        .map_err(database_error)?;
    let Some(result) = result else {
        return Ok(RpcResponse::new(1, empty_consume(&ctx.model)));
    };
    Ok(RpcResponse::new(
        0,
        ConsumeResponse {
            expire: 0,
            fault: fault(&ctx.model),
            acstatus: Some((!result.accepted).into()),
            autocharge: Some(result.autocharge.into()),
            balance: Some(result.balance),
        },
    ))
}

fn empty_consume(model: &str) -> ConsumeResponse {
    ConsumeResponse {
        expire: 0,
        fault: fault(model),
        acstatus: None,
        autocharge: None,
        balance: None,
    }
}

fn valid_card_id(value: &str) -> bool {
    value.len() == EACOIN_SESSION_ID_LENGTH && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn database_error(error: String) -> RpcError {
    RpcError::new(1, format!("database_error:{error}"))
}
