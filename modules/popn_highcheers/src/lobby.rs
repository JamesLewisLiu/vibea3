use vibea3::{Kbin, RpcContext, RpcError, RpcResult, rpc};

use crate::{
    State,
    database::{LobbyEntry, LobbyRoom},
    protocol::{LOBBY_LICENSE_DATA_COUNT, LOBBY_PLAYER_CAPACITY},
};

#[derive(Kbin)]
#[kbin(node = "lobby24")]
struct GetListRequest {
    location_id: String,
    net_version: u8,
}

#[derive(Kbin)]
#[kbin(node = "lobby24")]
struct GetListResponse {
    #[kbin(repeated)]
    list: Vec<ListItem>,
}

#[derive(Kbin)]
#[kbin(node = "list")]
struct ListItem {
    no: u32,
    time: u32,
    ip: u32,
    port: u16,
    local_ip: u32,
    music: i16,
    sheet: u8,
    is_ojama: u8,
    #[kbin(repeated)]
    gpm_id_list: Vec<String>,
    matching_num: u8,
    staff: i8,
    item_type: i16,
    item_id: i16,
    is_random: i8,
    #[kbin(array)]
    license_data: Vec<i16>,
    is_ranking: i8,
}

impl From<LobbyRoom> for ListItem {
    fn from(room: LobbyRoom) -> Self {
        Self {
            no: room.no,
            time: room.time,
            ip: room.ip,
            port: room.port,
            local_ip: room.local_ip,
            music: room.music,
            sheet: room.sheet,
            is_ojama: room.is_ojama,
            gpm_id_list: room
                .gpm_ids()
                .into_iter()
                .take(LOBBY_PLAYER_CAPACITY)
                .collect(),
            matching_num: room.matched_cnt,
            staff: room.staff,
            item_type: room.item_type,
            item_id: room.item_id,
            is_random: room.is_random,
            license_data: normalize(room.license_data, LOBBY_LICENSE_DATA_COUNT, -1),
            is_ranking: room.is_ranking,
        }
    }
}

#[rpc("lobby24.getList")]
async fn get_list(ctx: RpcContext<State>, request: GetListRequest) -> RpcResult<GetListResponse> {
    let list = ctx
        .state
        .database
        .lobby_rooms(
            &request.location_id,
            request.net_version,
            ctx.tag.as_deref(),
        )
        .await
        .map_err(database_error)?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(GetListResponse { list })
}

#[derive(Kbin)]
#[kbin(node = "lobby24")]
struct EntryRequest {
    ip: u32,
    local_ip: u32,
    time: u32,
    port: u16,
    music: i16,
    sheet: u8,
    is_ojama: u8,
    location_id: String,
    net_version: u8,
    gpm_id: String,
    staff: i8,
    item_type: i16,
    item_id: i16,
    is_random: i8,
    #[kbin(array)]
    license_data: Vec<i16>,
    is_ranking: i8,
}

#[derive(Kbin)]
#[kbin(node = "lobby24")]
struct EntryResponse {
    no: u32,
}

#[rpc("lobby24.entry")]
async fn entry(ctx: RpcContext<State>, request: EntryRequest) -> RpcResult<EntryResponse> {
    let no = ctx
        .state
        .database
        .enter_lobby(LobbyEntry {
            tag: ctx.tag.unwrap_or_default(),
            ip: request.ip,
            local_ip: request.local_ip,
            time: request.time,
            port: request.port,
            music: request.music,
            sheet: request.sheet,
            is_ojama: request.is_ojama,
            location_id: request.location_id,
            net_version: request.net_version,
            gpm_id: request.gpm_id,
            staff: request.staff,
            item_type: request.item_type,
            item_id: request.item_id,
            is_random: request.is_random,
            license_data: normalize(request.license_data, LOBBY_LICENSE_DATA_COUNT, -1),
            is_ranking: request.is_ranking,
        })
        .await
        .map_err(database_error)?;
    Ok(EntryResponse { no })
}

#[derive(Kbin)]
#[kbin(node = "lobby24")]
struct UpdateRequest {
    room_no: u32,
    matched_cnt: u8,
    location_id: String,
    gpm_id: String,
    staff: i8,
}

#[derive(Kbin)]
#[kbin(node = "lobby24")]
struct EmptyResponse {}

#[rpc("lobby24.update")]
async fn update(ctx: RpcContext<State>, request: UpdateRequest) -> RpcResult<EmptyResponse> {
    ctx.state
        .database
        .update_lobby(
            request.room_no,
            request.matched_cnt,
            &request.location_id,
            &request.gpm_id,
            request.staff,
        )
        .await
        .map_err(database_error)?;
    Ok(EmptyResponse {})
}

#[derive(Kbin)]
#[kbin(node = "lobby24")]
struct DeleteRequest {
    no: u32,
}

#[rpc("lobby24.delete")]
async fn delete(ctx: RpcContext<State>, request: DeleteRequest) -> RpcResult<EmptyResponse> {
    ctx.state
        .database
        .delete_lobby(request.no)
        .await
        .map_err(database_error)?;
    Ok(EmptyResponse {})
}

fn normalize<T: Clone>(mut values: Vec<T>, length: usize, fill: T) -> Vec<T> {
    values.truncate(length);
    values.resize(length, fill);
    values
}

fn database_error(error: String) -> RpcError {
    RpcError::new(1, format!("database_error:{error}"))
}

#[cfg(test)]
mod tests {
    use bson::DateTime;
    use vibea3::{EncodeOptions, encode_xml};

    use super::*;

    #[test]
    fn gpm_id_psmap_indexes_are_repeated_wire_nodes() {
        let item = ListItem::from(LobbyRoom {
            no: 1,
            tag: "self".into(),
            ip: 1,
            local_ip: 2,
            time: 60_000,
            port: 1234,
            music: 10,
            sheet: 2,
            is_ojama: 0,
            location_id: "JP-1".into(),
            net_version: 1,
            gpm_id: "000000000001".into(),
            gpm_ids: vec!["000000000001".into(), "000000000002".into()],
            matched_cnt: 2,
            staff: 0,
            item_type: -1,
            item_id: -1,
            is_random: 0,
            license_data: vec![-1; LOBBY_LICENSE_DATA_COUNT],
            is_ranking: 0,
            updated_at: DateTime::now(),
            expires_at: DateTime::now(),
        });
        let xml = encode_xml(&item, EncodeOptions::default()).unwrap();
        let text = String::from_utf8(xml).unwrap();
        assert_eq!(text.matches("<gpm_id_list ").count(), 2);
        assert!(!text.contains("gpm_id_list#"));
        assert!(text.contains("<gpm_id_list __type=\"str\">000000000001</gpm_id_list>"));
        assert!(text.contains("license_data __type=\"s16\" __count=\"20\""));
    }
}
