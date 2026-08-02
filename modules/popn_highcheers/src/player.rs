mod event;
mod methods;
mod music;
mod profile;
mod profile_write;
mod social;
mod stage;
mod start;
mod state;
#[cfg(test)]
mod tests;

use vibea3::{RpcContext, RpcError, RpcResult, rpc};

use crate::{State, protocol::FIXED_PLAYER_TEXT_PAYLOAD_BYTES};
use methods::*;
use music::*;
use profile::ReadResponse;
use profile_write::{ProfileWrite, patch};
use social::*;
use start::StartResponse;

#[rpc("player.read")]
async fn read(ctx: RpcContext<State>, request: ReadRequest) -> RpcResult<ReadResponse> {
    let _ = request.pref;
    let profile = ctx
        .state
        .database
        .profile(&request.ref_id, &request.data_id)
        .await
        .map_err(database_error)?;
    Ok(profile
        .as_ref()
        .map(ReadResponse::profile)
        .unwrap_or_else(ReadResponse::missing))
}

#[rpc("player.new")]
async fn new(ctx: RpcContext<State>, request: NewRequest) -> RpcResult<ReadResponse> {
    let profile = ctx
        .state
        .database
        .create_profile(
            &request.ref_id,
            &request.data_id,
            player_name(request.name),
            request.pref,
        )
        .await
        .map_err(database_error)?;
    Ok(profile
        .as_ref()
        .map(ReadResponse::profile)
        .unwrap_or_else(ReadResponse::missing))
}

#[rpc("player.conversion")]
async fn conversion(ctx: RpcContext<State>, request: ConversionRequest) -> RpcResult<ReadResponse> {
    let _ = (request.shop_name, request.chara);
    let profile = ctx
        .state
        .database
        .create_profile(
            &request.ref_id,
            &request.data_id,
            player_name(request.name),
            request.pref,
        )
        .await
        .map_err(database_error)?;
    Ok(profile
        .as_ref()
        .map(ReadResponse::profile)
        .unwrap_or_else(ReadResponse::missing))
}

#[rpc("player.write")]
async fn write(ctx: RpcContext<State>, request: WriteRequest) -> RpcResult<EmptyResponse> {
    let play_id = request.account.as_ref().and_then(|account| account.play_id);
    let start_type = request
        .account
        .as_ref()
        .and_then(|account| account.start_type);
    let stages = request.stage.iter().map(Into::into).collect();
    ctx.state
        .database
        .patch_profile(
            &request.ref_id,
            &request.data_id,
            patch(
                ProfileWrite {
                    account: request.account,
                    info: request.info,
                    customize: request.customize,
                    option: request.option,
                    config: request.config,
                    items: request.item,
                    characters: request.chara_param,
                    extra: request.ex_info,
                    netvs: request.netvs,
                    event: request.event_p29,
                },
                ctx.state.info.as_ref(),
            ),
        )
        .await
        .map_err(database_error)?;
    if let Some(play_id) = play_id {
        ctx.state
            .database
            .save_play(
                &request.ref_id,
                &request.data_id,
                play_id,
                start_type,
                request.shop_name,
                request.pref,
                stages,
            )
            .await
            .map_err(database_error)?;
    }
    Ok(EmptyResponse {})
}

#[rpc("player.start")]
async fn start(ctx: RpcContext<State>, request: StartRequest) -> RpcResult<StartResponse> {
    let _ = (
        request.loc_id,
        request.start_type,
        request.pcb_card.card_enable,
        request.pcb_card.card_soldout,
    );
    let play_id = ctx
        .state
        .database
        .begin_play(&request.ref_id, &request.data_id)
        .await
        .map_err(database_error)?;
    let usage = ctx
        .state
        .database
        .usage_snapshot(Some((&request.ref_id, &request.data_id)))
        .await
        .map_err(database_error)?;
    Ok(StartResponse::new(play_id, ctx.state.info.as_ref(), &usage))
}

#[rpc("player.read_score")]
async fn read_score(
    ctx: RpcContext<State>,
    request: ReadScoreRequest,
) -> RpcResult<ReadScoreResponse> {
    let _ = (request.pref, request.no);
    let music = ctx
        .state
        .database
        .scores(&request.ref_id, &request.data_id)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(ReadScoreResponse {
        con_flg: false,
        music,
    })
}

#[rpc("player.read_option")]
async fn read_option(
    ctx: RpcContext<State>,
    request: ReadOptionRequest,
) -> RpcResult<ReadOptionResponse> {
    let option = ctx
        .state
        .database
        .music_option(
            &request.ref_id,
            &request.data_id,
            request.music_num,
            request.sheet_num,
        )
        .await
        .map_err(database_error)?
        .map(Into::into);
    Ok(ReadOptionResponse { option })
}

#[rpc("player.write_music")]
async fn write_music(
    ctx: RpcContext<State>,
    request: WriteMusicRequest,
) -> RpcResult<EmptyResponse> {
    let ref_id = request.ref_id();
    let data_id = request.data_id();
    if !ref_id.is_empty() && ctx.state.info.contains_music(request.music_num) {
        ctx.state
            .database
            .save_score(
                &ref_id,
                &data_id,
                request.music_num,
                request.sheet_num,
                request.score,
                request.clear_type,
                request.clear_rank,
                request.chara_num,
                request.option(),
            )
            .await
            .map_err(database_error)?;
    }
    Ok(EmptyResponse {})
}

#[rpc("player.buy")]
async fn buy(ctx: RpcContext<State>, request: BuyRequest) -> RpcResult<EmptyResponse> {
    let _ = (
        request.play_id,
        request.id,
        request.item_type,
        request.param,
        request.price,
    );
    ctx.state
        .database
        .set_lumina(&request.ref_id, request.lumina)
        .await
        .map_err(database_error)?;
    Ok(EmptyResponse {})
}

#[rpc("player.tsumtsum")]
async fn tsumtsum(
    _ctx: RpcContext<State>,
    request: TsumTsumRequest,
) -> RpcResult<TsumTsumResponse> {
    let _ = (request.ref_id, request.uid);
    Ok(TsumTsumResponse { status: 0 })
}

#[rpc("player.friend")]
async fn friend(_ctx: RpcContext<State>, request: FriendRequest) -> RpcResult<FriendResponse> {
    let _ = (request.ref_id, request.data_id, request.no, request.tran_no);
    Ok(FriendResponse::disconnected())
}

#[rpc("player.update_ranking")]
async fn update_ranking(
    _ctx: RpcContext<State>,
    request: RankingRequest,
) -> RpcResult<RankingResponse> {
    let _ = request;
    Ok(RankingResponse::empty())
}

#[rpc("player.write_course")]
async fn write_course(ctx: RpcContext<State>, request: CourseRequest) -> RpcResult<EmptyResponse> {
    let ref_id = request.ref_id();
    let data_id = request.data_id();
    ctx.state
        .database
        .save_course(&ref_id, &data_id, request.into_record())
        .await
        .map_err(database_error)?;
    Ok(EmptyResponse {})
}

#[rpc("player.delete")]
async fn delete(ctx: RpcContext<State>, request: IdRequest) -> RpcResult<EmptyResponse> {
    ctx.state
        .database
        .delete_profile(&request.rid)
        .await
        .map_err(database_error)?;
    Ok(EmptyResponse {})
}

#[rpc("player.end")]
async fn end(_ctx: RpcContext<State>, request: IdRequest) -> RpcResult<EmptyResponse> {
    let _ = request.rid;
    Ok(EmptyResponse {})
}

#[rpc("player.logout")]
async fn logout(_ctx: RpcContext<State>, request: LogoutRequest) -> RpcResult<EmptyResponse> {
    let _ = (request.ref_id, request.data_id);
    Ok(EmptyResponse {})
}

pub(crate) fn database_error(error: String) -> RpcError {
    RpcError::new(1, format!("database_error:{error}"))
}

fn player_name(value: String) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if output.len() + character.len_utf8() > FIXED_PLAYER_TEXT_PAYLOAD_BYTES {
            break;
        }
        output.push(character);
    }
    output
}
