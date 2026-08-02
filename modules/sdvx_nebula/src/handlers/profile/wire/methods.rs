use super::super::*;
use super::*;

#[rpc("game.sv7_new")]
async fn new_profile(ctx: RpcContext<State>, request: NewRequest) -> RpcResult<EmptyResponse> {
    let user_id = ctx
        .state
        .database
        .user_id(&request.refid, Some(&request.dataid))
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)?;
    ctx.state
        .database
        .create_profile(&PlayerProfile::new(user_id, request.name))
        .await
        .map_err(database_error)?;
    Ok(EmptyResponse {})
}

#[rpc("game.sv7_load")]
async fn load(ctx: RpcContext<State>, request: LoadRequest) -> RpcResult<LoadResponse> {
    let Some(user_id) = ctx
        .state
        .database
        .user_id(&request.refid, Some(&request.dataid))
        .await
        .map_err(database_error)?
    else {
        return Ok(LoadResponse::missing());
    };
    let Some(profile) = ctx
        .state
        .database
        .profile(&user_id)
        .await
        .map_err(database_error)?
    else {
        return Ok(LoadResponse::missing());
    };
    let scores = ctx
        .state
        .database
        .scores(&user_id)
        .await
        .map_err(database_error)?;
    Ok(LoadResponse::found(profile, scores))
}

#[rpc("game.sv7_load_m")]
async fn load_music(ctx: RpcContext<State>, request: RefIdRequest) -> RpcResult<LoadMusicResponse> {
    let user_id = ctx
        .state
        .database
        .user_id(&request.refid, None)
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)?;
    Ok(LoadMusicResponse {
        music: MusicContainer {
            info: ctx
                .state
                .database
                .scores(&user_id)
                .await
                .map_err(database_error)?
                .into_iter()
                .map(Into::into)
                .collect(),
        },
        automation: Vec::new(),
    })
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct LoadMusicResponse {
    music: MusicContainer,
    #[kbin(repeated)]
    automation: Vec<Automation>,
}

#[rpc("game.sv7_load_r")]
async fn load_rivals(ctx: RpcContext<State>, request: RefIdRequest) -> RpcResult<RivalResponse> {
    if ctx
        .state
        .database
        .user_id(&request.refid, None)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(invalid_session());
    }
    let _ = RIVAL_MUSIC_PARAM_COUNT;
    Ok(RivalResponse {
        rival: Vec::new(),
        music: Vec::new(),
    })
}

#[rpc("game.sv7_load_ap")]
async fn load_automation(
    ctx: RpcContext<State>,
    request: RefIdUnderscoreRequest,
) -> RpcResult<AutomationResponse> {
    if ctx
        .state
        .database
        .user_id(&request.ref_id, None)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(invalid_session());
    }
    Ok(AutomationResponse {
        result: 0,
        automation: None,
    })
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct BuyRequest {
    refid: String,
    catalog_type: Option<u8>,
    catalog_id: Option<u32>,
    earned_gamecoin_packet: Option<i32>,
    earned_gamecoin_block: Option<i32>,
    currency_type: Option<u32>,
    item: Option<BuyItems>,
}
#[derive(Kbin)]
#[kbin(node = "item")]
struct BuyItems {
    #[kbin(array)]
    item_type: Option<Vec<i32>>,
    #[kbin(array)]
    item_id: Option<Vec<i32>>,
    #[kbin(array)]
    param: Option<Vec<i32>>,
    #[kbin(array)]
    price: Option<Vec<i32>>,
}
#[derive(Kbin)]
#[kbin(node = "game")]
struct BuyResponse {
    gamecoin_packet: u32,
    gamecoin_block: u32,
    result: i8,
}

#[rpc("game.sv7_buy")]
async fn buy(ctx: RpcContext<State>, request: BuyRequest) -> RpcResult<BuyResponse> {
    let user_id = ctx
        .state
        .database
        .user_id(&request.refid, None)
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)?;
    let mut profile = ctx
        .state
        .database
        .profile(&user_id)
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)?;
    if let Some(value) = request.earned_gamecoin_packet {
        profile.gamecoin_packet = profile.gamecoin_packet.saturating_add_signed(value);
    }
    if let Some(value) = request.earned_gamecoin_block {
        profile.gamecoin_block = profile.gamecoin_block.saturating_add_signed(value);
    }
    let mut result = crate::protocol::BUY_RESULT_SUCCESS;
    if let Some(items) = request.item {
        let types = items.item_type.unwrap_or_default();
        let ids = items.item_id.unwrap_or_default();
        let params = items.param.unwrap_or_default();
        let cost = items
            .price
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| u32::try_from(value).ok())
            .fold(0_u32, u32::saturating_add);
        let balance = match request.currency_type {
            Some(crate::protocol::CURRENCY_GAMECOIN_PACKET) => &mut profile.gamecoin_packet,
            Some(crate::protocol::CURRENCY_GAMECOIN_BLOCK) => &mut profile.gamecoin_block,
            _ => &mut profile.gamecoin_packet,
        };
        if *balance < cost {
            result = crate::protocol::BUY_RESULT_INSUFFICIENT_FUNDS;
        } else {
            *balance -= cost;
        }
        for (index, item_type) in types
            .into_iter()
            .enumerate()
            .filter(|_| result == crate::protocol::BUY_RESULT_SUCCESS)
        {
            if let Some(id) = ids.get(index)
                && let (Ok(item_type), Ok(id), Ok(param)) = (
                    u32::try_from(item_type),
                    u32::try_from(*id),
                    u32::try_from(params.get(index).copied().unwrap_or_default()),
                )
            {
                profile
                    .items
                    .retain(|item| !(item.item_type == item_type && item.id == id));
                profile.items.push(PlayerItem {
                    item_type,
                    id,
                    param,
                });
            }
        }
    }
    ctx.state
        .database
        .save_profile(&mut profile)
        .await
        .map_err(database_error)?;
    Ok(BuyResponse {
        gamecoin_packet: profile.gamecoin_packet,
        gamecoin_block: profile.gamecoin_block,
        result,
    })
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct SaveAutomationRequest {
    ref_id: String,
    loc_id: Option<String>,
    automation: Automation,
}
#[rpc("game.sv7_save_ap")]
async fn save_automation(
    ctx: RpcContext<State>,
    request: SaveAutomationRequest,
) -> RpcResult<AutomationResponse> {
    let user_id = ctx
        .state
        .database
        .user_id(&request.ref_id, None)
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)?;
    ctx.state.database.save_automation(&user_id,doc!{"mix_id":request.automation.mix_id,"mix_code":request.automation.mix_code.clone(),"mix_name":request.automation.mix_name.clone(),"generate_param":request.automation.generate_param.clone(),"tag_bit":request.automation.tag_bit,"jacket_id":request.automation.jacket_id,"etc":request.automation.etc.clone(),"updated_at":DateTime::now()}).await.map_err(database_error)?;
    Ok(AutomationResponse {
        result: 0,
        automation: None,
    })
}
