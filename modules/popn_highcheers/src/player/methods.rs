use vibea3::Kbin;

use super::event::EventWrite;
use super::profile_write::{AccountWrite, CustomizeWrite, InfoWrite};
use super::stage::StageWrite;
use super::state::{CharacterData, ConfigData, ExtraData, ItemData, NetvsWrite, OptionData};

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct ReadRequest {
    pub ref_id: String,
    pub data_id: String,
    pub pref: i8,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct NewRequest {
    pub ref_id: String,
    pub data_id: String,
    pub name: String,
    pub pref: i8,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct ConversionRequest {
    pub ref_id: String,
    pub data_id: String,
    pub name: String,
    pub shop_name: String,
    pub pref: i8,
    pub chara: i16,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct WriteRequest {
    pub ref_id: String,
    pub data_id: String,
    pub shop_name: String,
    pub pref: i8,
    pub account: Option<AccountWrite>,
    pub info: Option<InfoWrite>,
    #[kbin(repeated)]
    pub stage: Vec<StageWrite>,
    pub config: Option<ConfigData>,
    pub option: Option<OptionData>,
    #[kbin(repeated)]
    pub item: Vec<ItemData>,
    #[kbin(repeated)]
    pub chara_param: Vec<CharacterData>,
    pub customize: Option<CustomizeWrite>,
    pub netvs: Option<NetvsWrite>,
    #[kbin(repeated)]
    pub ex_info: Vec<ExtraData>,
    pub event_p29: Option<EventWrite>,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct StartRequest {
    #[kbin(attr)]
    pub loc_id: String,
    #[kbin(attr)]
    pub ref_id: String,
    #[kbin(attr)]
    pub data_id: String,
    #[kbin(attr)]
    pub start_type: String,
    pub pcb_card: PcbCard,
}

#[derive(Kbin)]
#[kbin(node = "pcb_card")]
pub(super) struct PcbCard {
    pub card_enable: i8,
    pub card_soldout: i8,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct IdRequest {
    pub rid: String,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct LogoutRequest {
    #[kbin(attr)]
    pub ref_id: String,
    #[kbin(attr)]
    pub data_id: String,
}

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct EmptyResponse {}
