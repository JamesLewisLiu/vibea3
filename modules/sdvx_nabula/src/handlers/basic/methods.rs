use super::*;

#[rpc("game.sv7_log")]
async fn log(_ctx: RpcContext<State>, _request: LogRequest) -> RpcResult<EmptyResponse> {
    Ok(EmptyResponse {})
}

#[rpc("game.sv7_exception")]
async fn exception(
    _ctx: RpcContext<State>,
    _request: ExceptionRequest,
) -> RpcResult<EmptyResponse> {
    Ok(EmptyResponse {})
}

#[rpc("game.sv7_entry_e")]
async fn entry_end(_ctx: RpcContext<State>, _request: EntryEndRequest) -> RpcResult<EmptyResponse> {
    Ok(EmptyResponse {})
}

#[rpc("game.sv7_entry_s")]
async fn entry_start(
    _ctx: RpcContext<State>,
    request: EntryStartRequest,
) -> RpcResult<EntryStartResponse> {
    Ok(EntryStartResponse {
        entry_id: request.entry_id.unwrap_or(1),
        entry: vec![EntryAddress {
            port: request.port.unwrap_or(0),
            gip: request.gip.unwrap_or(Ip4(Ipv4Addr::LOCALHOST)),
            lip: request.lip.unwrap_or(Ip4(Ipv4Addr::LOCALHOST)),
        }],
    })
}

#[rpc("game.sv7_frozen")]
async fn frozen(ctx: RpcContext<State>, request: FrozenRequest) -> RpcResult<FrozenResponse> {
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
    Ok(FrozenResponse { result: 0 })
}

#[rpc("game.sv7_hiscore")]
async fn hiscore(ctx: RpcContext<State>, request: HiscoreRequest) -> RpcResult<HiscoreResponse> {
    let _ = ctx
        .state
        .database
        .hiscores(
            request
                .limit
                .unwrap_or(crate::protocol::HISCORE_PAGE_DEFAULT),
            request.offset.unwrap_or(0),
        )
        .await
        .map_err(database_error)?;
    let _ = crate::protocol::HISCORE_LEVEL_BUCKET_COUNT;
    Ok(HiscoreResponse {
        sc: HiscoreScores { d: Vec::new() },
    })
}

#[rpc("game.sv7_lounge")]
async fn lounge(ctx: RpcContext<State>, _request: LoungeRequest) -> RpcResult<LoungeResponse> {
    Ok(LoungeResponse {
        interval: ctx
            .state
            .common
            .lounge_interval
            .max(DEFAULT_LOUNGE_INTERVAL_SECONDS),
        wait: Vec::new(),
    })
}

#[rpc("game.sv7_play_s")]
async fn play_start(
    ctx: RpcContext<State>,
    _request: EmptyRequest,
) -> RpcResult<PlayStartResponse> {
    let play_id = ctx
        .state
        .database
        .next_play_id()
        .await
        .map_err(database_error)?;
    Ok(PlayStartResponse {
        play_id: u32::try_from(play_id).unwrap_or(u32::MAX),
    })
}

#[rpc("game.sv7_play_e")]
async fn play_end(ctx: RpcContext<State>, request: PlayEndRequest) -> RpcResult<EmptyResponse> {
    let user_id = ctx
        .state
        .database
        .user_id(&request.refid, None)
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)?;
    if let Some(play_id) = request.play_id {
        ctx.state
            .database
            .end_play(i64::from(play_id))
            .await
            .map_err(database_error)?;
    }
    if request.play_id.is_none() {
        let play_id = ctx
            .state
            .database
            .next_play_id()
            .await
            .map_err(database_error)?;
        ctx.state
            .database
            .start_play(&PlayRecord {
                play_id,
                user_id,
                location_id: request.locid.unwrap_or_default(),
                started_at: DateTime::now(),
                ended_at: Some(DateTime::now()),
            })
            .await
            .map_err(database_error)?;
    }
    Ok(EmptyResponse {})
}

#[rpc("game.sv7_sample")]
async fn sample(ctx: RpcContext<State>, _request: SampleRequest) -> RpcResult<SampleResponse> {
    Ok(SampleResponse {
        release: ctx.state.common.release_code.clone(),
    })
}

#[rpc("game.sv7_serial")]
async fn serial(ctx: RpcContext<State>, request: SerialRequest) -> RpcResult<SerialResponse> {
    let user_id = ctx
        .state
        .database
        .user_id(&request.refid, None)
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)?;
    let profile = ctx
        .state
        .database
        .profile(&user_id)
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)?;
    Ok(SerialResponse {
        serial_name: request.code,
        item: Vec::new(),
        gamecoin_packet: profile.gamecoin_packet,
        gamecoin_block: profile.gamecoin_block,
        result: 0,
    })
}

#[rpc("game.sv7_shop")]
async fn shop(ctx: RpcContext<State>, request: ShopRequest) -> RpcResult<ShopResponse> {
    let location = ctx.srcid.as_deref().unwrap_or("unknown");
    ctx.state
        .database
        .save_cabinet(
            location,
            doc! { "etc": request.etc, "updated_at": DateTime::now() },
        )
        .await
        .map_err(database_error)?;
    Ok(ShopResponse {
        nxt_time: DEFAULT_SHOP_NEXT_TIME_SECONDS,
    })
}

#[rpc("game.sv7_save_usta_link")]
async fn save_usta_link(
    ctx: RpcContext<State>,
    request: UstaLinkRequest,
) -> RpcResult<UstaLinkResponse> {
    if ctx
        .state
        .database
        .user_id(&request.usta_link.ref_id, None)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(invalid_session());
    }
    Ok(UstaLinkResponse {
        usta_link: UstaLinkResult {
            link_id: request.usta_link.ref_id,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibea3::{DecodeOptions, EncodeOptions, NameMode, decode_kbin, encode_kbin, encode_xml};

    #[test]
    fn required_empty_containers_and_attributes_have_the_client_shape() {
        let hiscore = HiscoreResponse {
            sc: HiscoreScores { d: Vec::new() },
        };
        let xml = String::from_utf8(
            encode_xml(
                &hiscore,
                EncodeOptions {
                    encoding: 0xa0,
                    names: NameMode::Full,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(xml.contains("<sc"));
        let sample = SampleResponse {
            release: "2026071400".into(),
        };
        let xml = String::from_utf8(
            encode_xml(
                &sample,
                EncodeOptions {
                    encoding: 0xa0,
                    names: NameMode::Full,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(xml.contains("release=\"2026071400\""));
    }

    #[test]
    fn psmap_lounge_wait_is_repeated_nodes_not_an_array() {
        let response = LoungeResponse {
            interval: 10,
            wait: vec![LoungeWait { m_id: 42 }],
        };
        let packet = encode_kbin(
            &response,
            EncodeOptions {
                encoding: 0x80,
                names: NameMode::Packed,
            },
        )
        .unwrap();
        let decoded: LoungeResponse = decode_kbin(&packet, DecodeOptions::default()).unwrap();
        assert_eq!(decoded.wait.len(), 1);
        assert_eq!(decoded.wait[0].m_id, 42);
    }
}
