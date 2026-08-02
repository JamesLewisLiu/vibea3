use crate::{
    CARD_ID_HEX_LENGTH, CARD_PIN_DIGIT_COUNT, REF_ID_DIGIT_COUNT, State,
    database::{AuthResult, CardInquiry},
    fault,
    system::card_id_to_user_code,
};
use vibea3::{Kbin, RpcContext, RpcError, RpcResponse, RpcResult, rpc};

const DUMMY_REF_ID: &str = "EA00000000000000";
// AVS-EA3 card status values, verified against popn.dll's XRPC status converter.
const STATUS_OK: i32 = 0;
const STATUS_ERROR: i32 = 1;
const STATUS_BANNED: i32 = 109;
const STATUS_NOT_REGISTERED: i32 = 112;
const STATUS_BAD_PIN: i32 = 116;

#[derive(Kbin)]
#[kbin(node = "cardmng")]
struct InquireRequest {
    #[kbin(attr)]
    cardid: String,
    #[kbin(attr)]
    cardtype: Option<String>,
    #[kbin(attr)]
    update: Option<u8>,
}

#[derive(Kbin)]
#[kbin(node = "cardmng")]
struct CardResponse {
    #[kbin(attr)]
    binded: Option<u8>,
    #[kbin(attr)]
    dataid: Option<String>,
    #[kbin(attr)]
    ecflag: Option<u8>,
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    expired: Option<u8>,
    #[kbin(attr)]
    extidflag: Option<u8>,
    #[kbin(attr)]
    fault: Option<String>,
    #[kbin(attr)]
    lastupdate: Option<i32>,
    #[kbin(attr)]
    newflag: Option<u8>,
    #[kbin(attr)]
    pcode: Option<String>,
    #[kbin(attr)]
    refid: Option<String>,
    #[kbin(attr)]
    useridflag: Option<u8>,
}

#[rpc("cardmng.inquire")]
async fn inquire(
    ctx: RpcContext<State>,
    request: InquireRequest,
) -> RpcResult<RpcResponse<CardResponse>> {
    if !valid_card_id(&request.cardid) || ctx.model.is_empty() {
        return Ok(RpcResponse::new(STATUS_ERROR, empty_response(&ctx.model)));
    }
    if request.cardid == "0000000000000000" {
        let mut response = empty_response(&ctx.model);
        response.refid = Some(DUMMY_REF_ID.into());
        response.dataid = Some(DUMMY_REF_ID.into());
        response.pcode = card_id_to_user_code(&request.cardid);
        response.newflag = Some(1);
        response.binded = Some(0);
        response.expired = Some(0);
        response.ecflag = Some(1);
        response.useridflag = Some(1);
        response.extidflag = Some(1);
        response.lastupdate = Some(0);
        return Ok(RpcResponse::new(STATUS_OK, response));
    }
    let inquiry = ctx
        .state
        .database
        .inquire_card(
            request.cardid,
            ctx.model.clone(),
            ctx.tag.clone().unwrap_or_default(),
            request.update != Some(0),
        )
        .await
        .map_err(database_error)?;
    match inquiry {
        CardInquiry::Missing | CardInquiry::Inactive => Ok(RpcResponse::new(
            STATUS_NOT_REGISTERED,
            empty_response(&ctx.model),
        )),
        CardInquiry::Banned => Ok(RpcResponse::new(STATUS_BANNED, empty_response(&ctx.model))),
        CardInquiry::Active {
            ref_id,
            user_id,
            eacoin_enabled,
            bound,
        } => {
            let mut response = empty_response(&ctx.model);
            response.refid = Some(ref_id);
            response.dataid = Some(user_id);
            response.newflag = Some(0);
            response.binded = Some(bound.into());
            response.expired = Some(0);
            response.ecflag = Some((ctx.state.config.infinite_eacoin || eacoin_enabled).into());
            response.useridflag = Some(1);
            response.extidflag = Some(1);
            response.lastupdate = Some(0);
            Ok(RpcResponse::new(STATUS_OK, response))
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "cardmng")]
struct GetRefIdRequest {
    #[kbin(attr)]
    cardid: String,
    #[kbin(attr)]
    cardtype: Option<String>,
    #[kbin(attr)]
    newflag: Option<u8>,
    #[kbin(attr)]
    passwd: String,
}

#[rpc("cardmng.getrefid")]
async fn get_ref_id(
    ctx: RpcContext<State>,
    request: GetRefIdRequest,
) -> RpcResult<RpcResponse<CardResponse>> {
    if !valid_card_id(&request.cardid) || !valid_pin(&request.passwd) {
        return Ok(RpcResponse::new(STATUS_ERROR, empty_response(&ctx.model)));
    }
    let session = ctx
        .state
        .database
        .register_card(
            request.cardid,
            request.passwd,
            ctx.tag.clone().unwrap_or_default(),
        )
        .await
        .map_err(database_error)?;
    let Some(identity) = session else {
        return Ok(RpcResponse::new(STATUS_ERROR, empty_response(&ctx.model)));
    };
    let mut response = empty_response(&ctx.model);
    response.dataid = Some(identity.user_id);
    response.refid = Some(identity.ref_id);
    Ok(RpcResponse::new(STATUS_OK, response))
}

#[derive(Kbin)]
#[kbin(node = "cardmng")]
struct BindModelRequest {
    #[kbin(attr)]
    refid: String,
}

#[rpc("cardmng.bindmodel")]
async fn bind_model(
    ctx: RpcContext<State>,
    request: BindModelRequest,
) -> RpcResult<RpcResponse<CardResponse>> {
    if !valid_ref_id(&request.refid) {
        return Ok(RpcResponse::new(STATUS_ERROR, empty_response(&ctx.model)));
    }
    let Some(user_id) = ctx
        .state
        .database
        .session_user(request.refid)
        .await
        .map_err(database_error)?
    else {
        return Ok(RpcResponse::new(STATUS_ERROR, empty_response(&ctx.model)));
    };
    let mut response = empty_response(&ctx.model);
    response.dataid = Some(user_id);
    Ok(RpcResponse::new(STATUS_OK, response))
}

#[derive(Kbin)]
#[kbin(node = "cardmng")]
struct GetDataListRequest {}

#[rpc("cardmng.getdatalist")]
async fn get_data_list(
    ctx: RpcContext<State>,
    _request: GetDataListRequest,
) -> RpcResult<RpcResponse<CardResponse>> {
    Ok(RpcResponse::new(STATUS_OK, empty_response(&ctx.model)))
}

#[derive(Kbin)]
#[kbin(node = "cardmng")]
struct AuthPassRequest {
    #[kbin(attr)]
    pass: String,
    #[kbin(attr)]
    refid: String,
}

#[rpc("cardmng.authpass")]
async fn auth_pass(
    ctx: RpcContext<State>,
    request: AuthPassRequest,
) -> RpcResult<RpcResponse<CardResponse>> {
    if !valid_ref_id(&request.refid) || !valid_pin(&request.pass) {
        return Ok(RpcResponse::new(STATUS_ERROR, empty_response(&ctx.model)));
    }
    let status = match ctx
        .state
        .database
        .authenticate(request.refid, request.pass)
        .await
        .map_err(database_error)?
    {
        AuthResult::Valid => STATUS_OK,
        AuthResult::Invalid => STATUS_BAD_PIN,
        AuthResult::Missing => STATUS_ERROR,
    };
    Ok(RpcResponse::new(status, empty_response(&ctx.model)))
}

fn empty_response(model: &str) -> CardResponse {
    CardResponse {
        binded: None,
        dataid: None,
        ecflag: None,
        expire: 0,
        expired: None,
        extidflag: None,
        fault: fault(model),
        lastupdate: None,
        newflag: None,
        pcode: None,
        refid: None,
        useridflag: None,
    }
}

fn valid_card_id(value: &str) -> bool {
    value.len() == CARD_ID_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_pin(value: &str) -> bool {
    value.len() == CARD_PIN_DIGIT_COUNT && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_ref_id(value: &str) -> bool {
    value.len() == REF_ID_DIGIT_COUNT && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn database_error(error: String) -> RpcError {
    RpcError::new(1, format!("database_error:{error}"))
}
