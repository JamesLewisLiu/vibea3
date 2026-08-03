use super::*;

#[rpc("game.sv7_save")]
async fn save(ctx: RpcContext<State>, request: SaveRequest) -> RpcResult<EmptyResponse> {
    validate_tracks(&request.track)?;
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
    if let Some(v) = request.appeal_id {
        profile.appeal_id = v
    }
    if let Some(v) = request.skill_level {
        profile.skill_level = v
    }
    if let Some(v) = request.skill_name_id {
        profile.skill_name_id = v
    }
    if let Some(v) = request.skill_type {
        profile.skill_type = Some(v)
    }
    if let Some(v) = request.earned_gamecoin_packet {
        profile.gamecoin_packet = profile.gamecoin_packet.saturating_add_signed(v)
    }
    if let Some(v) = request.earned_gamecoin_block {
        profile.gamecoin_block = profile.gamecoin_block.saturating_add_signed(v)
    }
    if let Some(v) = request.earned_blaster_energy {
        profile.blaster_energy = profile.blaster_energy.saturating_add_signed(v)
    }
    if let Some(v) = request.variant_gate {
        if let Some(power) = v.earned_power {
            profile.variant_power = profile.variant_power.saturating_add(power)
        }
        if let Some(e) = v.earned_element {
            apply_elements(&mut profile.variant_elements, e)
        }
        if let Some(radar) = v.over_radar {
            profile.over_radar = radar
        }
    }
    if let Some(setting) = request.setting {
        setting.apply(&mut profile.setting)
    }
    if let Some(items) = request.item {
        for item in items.info {
            if let Some(id) = item.id {
                profile
                    .items
                    .retain(|old| old.item_type != item.item_type || old.id != id);
                profile.items.push(PlayerItem {
                    item_type: item.item_type,
                    id,
                    param: item.param.unwrap_or_default(),
                });
            }
        }
    }
    if let Some(params) = request.param {
        for param in params.info {
            let mut values = param.param;
            values.truncate(PARAMETER_VALUE_COUNT);
            profile
                .params
                .retain(|old| old.param_type != param.param_type || old.id != param.id);
            profile.params.push(PlayerParam {
                param_type: param.param_type,
                id: param.id,
                values,
            });
        }
    }
    if let Some(stories) = request.story {
        for story in stories.info {
            profile.stories.retain(|old| old.story_id != story.story_id);
            profile.stories.push(StoryProgress {
                story_id: story.story_id,
                progress_id: story.progress_id,
                progress_param: story.progress_param,
                clear_cnt: story.clear_cnt,
                route_flg: story.route_flg,
            });
        }
    }
    for track in &request.track {
        ctx.state
            .database
            .upsert_score(&track.score(&user_id))
            .await
            .map_err(database_error)?;
    }
    profile.play_count = profile.play_count.saturating_add(1);
    profile.today_count = profile.today_count.saturating_add(1);
    profile.week_play_count = profile.week_play_count.saturating_add(1);
    ctx.state
        .database
        .save_profile(&mut profile)
        .await
        .map_err(database_error)?;
    if let Some(play_id) = request.play_id {
        ctx.state
            .database
            .end_play(i64::from(play_id))
            .await
            .map_err(database_error)?;
    }
    Ok(EmptyResponse {})
}

fn apply_elements(value: &mut RadarElements, delta: ElementWrite) {
    if let Some(v) = delta.notes {
        value.notes = value.notes.saturating_add(v)
    }
    if let Some(v) = delta.peak {
        value.peak = value.peak.saturating_add(v)
    }
    if let Some(v) = delta.tsumami {
        value.tsumami = value.tsumami.saturating_add(v)
    }
    if let Some(v) = delta.tricky {
        value.tricky = value.tricky.saturating_add(v)
    }
    if let Some(v) = delta.onehand {
        value.onehand = value.onehand.saturating_add(v)
    }
    if let Some(v) = delta.handtrip {
        value.handtrip = value.handtrip.saturating_add(v)
    }
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct SaveMusicRequest {
    refid: String,
    dataid: Option<String>,
    locid: Option<String>,
    #[kbin(repeated)]
    track: Vec<TrackWrite>,
}
#[rpc("game.sv7_save_m")]
async fn save_music(ctx: RpcContext<State>, request: SaveMusicRequest) -> RpcResult<EmptyResponse> {
    validate_tracks(&request.track)?;
    let user_id = ctx
        .state
        .database
        .user_id(&request.refid, request.dataid.as_deref())
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)?;
    for track in request.track {
        ctx.state
            .database
            .upsert_score(&track.score(&user_id))
            .await
            .map_err(database_error)?;
    }
    Ok(EmptyResponse {})
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct SaveCourseRequest {
    refid: String,
    play_id: Option<u32>,
    ssnid: Option<i16>,
    crsid: Option<i16>,
    st: Option<i16>,
    sc: Option<u32>,
    ex: Option<u32>,
    ct: Option<i16>,
    gr: Option<i16>,
    jr: Option<u32>,
    cr: Option<u32>,
    nr: Option<u32>,
    er: Option<u32>,
    cm: Option<u32>,
    ar: Option<i16>,
    locid: Option<String>,
    #[kbin(repeated)]
    tr: Vec<CourseTrack>,
    kac_id: Option<String>,
}
#[rpc("game.sv7_save_c")]
async fn save_course(
    ctx: RpcContext<State>,
    request: SaveCourseRequest,
) -> RpcResult<EmptyResponse> {
    ensure_ref(&ctx, &request.refid).await?;
    Ok(EmptyResponse {})
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct SaveEventRequest {
    locid: Option<String>,
    cardnumber: Option<String>,
    refid: String,
    playid: Option<i32>,
    is_paseli: Option<bool>,
    online_num: Option<i32>,
    local_num: Option<i32>,
    start_option: Option<i32>,
    print_num: Option<i32>,
    start_time: Option<u64>,
    valgene_num: Option<i32>,
    campaign: Option<CampaignWrite>,
    #[kbin(repeated)]
    something: Vec<RankingWrite>,
    played_music: Option<PlayedMusic>,
    #[kbin(repeated)]
    weekly_music: Vec<WeeklyMusicWrite>,
}
#[derive(Kbin)]
#[kbin(node = "campaign")]
struct CampaignWrite {
    id: Option<i32>,
    cnt: Option<i32>,
}
#[derive(Kbin)]
#[kbin(node = "something")]
struct RankingWrite {
    ranking_id: Option<i32>,
    value: Option<i64>,
}
#[derive(Kbin)]
#[kbin(node = "played_music")]
struct PlayedMusic {
    #[kbin(repeated)]
    music: Vec<PlayedMusicInfo>,
}
#[derive(Kbin)]
#[kbin(node = "music")]
struct PlayedMusicInfo {
    #[kbin(attr, rename = "id")]
    id: Option<String>,
    #[kbin(attr, rename = "type")]
    music_type: Option<String>,
}
#[derive(Kbin)]
#[kbin(node = "weekly_music")]
struct WeeklyMusicWrite {
    week_id: Option<i32>,
    music_id: Option<i32>,
    music_type: Option<i32>,
    exscore: Option<i32>,
    play_cnt: Option<i32>,
    hiscore_cnt: Option<i32>,
}
#[derive(Kbin)]
#[kbin(node = "game")]
struct SaveEventResponse {
    pb_infection: Option<PbInfection>,
    #[kbin(repeated)]
    weekly_music: Vec<WeeklyMusicResponse>,
}
#[derive(Kbin)]
#[kbin(node = "pb_infection")]
struct PbInfection {
    packet: CurrencyDelta,
    block: CurrencyDelta,
}
#[derive(Kbin)]
#[kbin(node = "packet")]
struct CurrencyDelta {
    before: i32,
    after: i32,
}
#[derive(Kbin)]
#[kbin(node = "weekly_music")]
struct WeeklyMusicResponse {
    week_id: i32,
    music_id: i32,
    music_type: i32,
    exscore: i32,
    rank: i32,
}
#[rpc("game.sv7_save_e")]
async fn save_event(
    ctx: RpcContext<State>,
    request: SaveEventRequest,
) -> RpcResult<SaveEventResponse> {
    ensure_ref(&ctx, &request.refid).await?;
    Ok(SaveEventResponse {
        pb_infection: None,
        weekly_music: request
            .weekly_music
            .into_iter()
            .map(|v| WeeklyMusicResponse {
                week_id: v.week_id.unwrap_or_default(),
                music_id: v.music_id.unwrap_or_default(),
                music_type: v.music_type.unwrap_or_default(),
                exscore: v.exscore.unwrap_or_default(),
                rank: 0,
            })
            .collect(),
    })
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct EnergyRequest {
    refid: String,
    id: i32,
    energy: i32,
}
#[derive(Kbin)]
#[kbin(node = "game")]
struct EnergyResponse {
    after: i32,
    result: i8,
}
#[rpc("game.sv7_save_fi")]
async fn save_floor_infection(
    ctx: RpcContext<State>,
    request: EnergyRequest,
) -> RpcResult<EnergyResponse> {
    ensure_ref(&ctx, &request.refid).await?;
    Ok(EnergyResponse {
        after: request.energy,
        result: 0,
    })
}
#[rpc("game.sv7_save_pb")]
async fn save_policy_break(
    ctx: RpcContext<State>,
    request: EnergyRequest,
) -> RpcResult<EnergyResponse> {
    ensure_ref(&ctx, &request.refid).await?;
    Ok(EnergyResponse {
        after: request.energy,
        result: 0,
    })
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct MegaRequest {
    refid: String,
    megamix_id: Option<u32>,
    play_id: Option<u32>,
    #[kbin(array)]
    music_id: Option<Vec<i32>>,
    #[kbin(array)]
    music_type: Option<Vec<i32>>,
    #[kbin(array)]
    score: Option<Vec<i32>>,
    #[kbin(array)]
    star: Option<Vec<i32>>,
    #[kbin(array)]
    gear_level: Option<Vec<i32>>,
    result: Option<u8>,
    locid: Option<String>,
    etc: Option<String>,
}
#[rpc("game.sv7_save_mega")]
async fn save_mega(ctx: RpcContext<State>, request: MegaRequest) -> RpcResult<EmptyResponse> {
    ensure_ref(&ctx, &request.refid).await?;
    Ok(EmptyResponse {})
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct CampaignRequest {
    locid: Option<String>,
    volte_factory: Option<FactoryCampaign>,
    reitaisai2018: Option<GenericCampaign>,
}
#[derive(Kbin)]
#[kbin(node = "volte_factory")]
struct FactoryCampaign {
    info: Option<FactoryCampaignInfo>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct FactoryCampaignInfo {
    factory_id: Option<u8>,
    goods_id: Option<i32>,
    num: Option<i32>,
}
#[derive(Kbin)]
#[kbin(node = "reitaisai2018")]
struct GenericCampaign {
    id: Option<i32>,
    before: Option<i32>,
    after: Option<i32>,
    num: Option<i32>,
}
#[derive(Kbin)]
#[kbin(node = "game")]
struct ResultResponse {
    result: i32,
}
#[rpc("game.sv7_save_campaign")]
async fn save_campaign(
    _ctx: RpcContext<State>,
    _request: CampaignRequest,
) -> RpcResult<ResultResponse> {
    Ok(ResultResponse { result: 0 })
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct ValgeneRequest {
    refid: String,
    valgene_id: Option<u32>,
    play_id: Option<u32>,
    consume_type: Option<i32>,
    price: Option<i32>,
    use_ticket: Option<bool>,
    item: Option<ValgeneItems>,
    locid: Option<String>,
}
#[derive(Kbin)]
#[kbin(node = "item")]
struct ValgeneItems {
    #[kbin(repeated)]
    info: Vec<ValgeneItem>,
}
#[derive(Kbin)]
#[kbin(node = "info")]
struct ValgeneItem {
    id: Option<u32>,
    #[kbin(rename = "type")]
    item_type: Option<u32>,
    param: Option<u32>,
}
#[derive(Kbin)]
#[kbin(node = "game")]
struct ValgeneResponse {
    result: i32,
    ticket_num: i32,
    limit_date: u64,
}
#[rpc("game.sv7_save_valgene")]
async fn save_valgene(
    ctx: RpcContext<State>,
    request: ValgeneRequest,
) -> RpcResult<ValgeneResponse> {
    ensure_ref(&ctx, &request.refid).await?;
    Ok(ValgeneResponse {
        result: 0,
        ticket_num: 0,
        limit_date: 0,
    })
}

async fn ensure_ref(ctx: &RpcContext<State>, refid: &str) -> RpcResult<String> {
    ctx.state
        .database
        .user_id(refid, None)
        .await
        .map_err(database_error)?
        .ok_or_else(invalid_session)
}

fn validate_tracks(tracks: &[TrackWrite]) -> RpcResult<()> {
    for track in tracks {
        if track
            .judge
            .as_ref()
            .is_some_and(|values| values.len() != crate::protocol::TRACK_JUDGE_COUNT)
        {
            return Err(vibea3::RpcError::new(
                1,
                "invalid_request:track judge must contain seven s32 values",
            ));
        }
        if track.matching.len() > crate::protocol::TRACK_MATCHING_PLAYER_COUNT {
            return Err(vibea3::RpcError::new(
                1,
                "invalid_request:track matching exceeds three players",
            ));
        }
    }
    Ok(())
}
