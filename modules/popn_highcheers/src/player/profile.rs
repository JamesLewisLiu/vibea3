use vibea3::Kbin;

use super::event::EventResponse;
use super::social::CourseData;
use super::state::{CharacterData, ConfigData, ExtraData, ItemData, OptionData};
use crate::database::{NetvsState, PlayerProfile};
use crate::protocol::{
    CLEAR_MEDAL_TYPE_COUNT, CUSTOMIZE_FIELD_COUNT, FAVORITE_CHARACTER_HISTORY_COUNT,
    LATEST_MUSIC_HISTORY_COUNT, NETVS_DIALOG_COUNT, NETVS_OJAMA_CONDITION_COUNT,
    NETVS_RECORD_COUNT, NETVS_SET_COUNT, NICE_HISTORY_COUNT, POWER_POINT_HISTORY_COUNT,
};

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct ReadResponse {
    pub result: i8,
    pub name: Option<String>,
    pub chara: Option<i16>,
    pub con_type: Option<i8>,
    #[kbin(repeated)]
    pub medal_cnt: Vec<MedalCount>,
    pub account: Option<AccountResponse>,
    pub eaappli: Option<EaAppliResponse>,
    pub info: Option<InfoResponse>,
    pub config: Option<ConfigData>,
    pub option: Option<OptionData>,
    #[kbin(repeated)]
    pub item: Vec<ItemData>,
    #[kbin(repeated)]
    pub chara_param: Vec<CharacterData>,
    #[kbin(repeated)]
    pub chara_param_old: Vec<CharacterData>,
    pub customize: Option<CustomizeResponse>,
    pub netvs: Option<NetvsResponse>,
    #[kbin(repeated)]
    pub course_data: Vec<CourseData>,
    #[kbin(repeated)]
    pub ex_info: Vec<ExtraData>,
    pub event_p29: Option<EventResponse>,
}

impl ReadResponse {
    pub(super) fn missing() -> Self {
        Self {
            result: 1,
            name: Some(String::new()),
            chara: Some(0),
            con_type: Some(0),
            medal_cnt: (0..CLEAR_MEDAL_TYPE_COUNT)
                .map(|clear_type| MedalCount { clear_type, cnt: 0 })
                .collect(),
            account: None,
            eaappli: None,
            info: None,
            config: None,
            option: None,
            item: Vec::new(),
            chara_param: Vec::new(),
            chara_param_old: Vec::new(),
            customize: None,
            netvs: None,
            course_data: Vec::new(),
            ex_info: Vec::new(),
            event_p29: None,
        }
    }

    pub(super) fn profile(profile: &PlayerProfile) -> Self {
        Self {
            result: 0,
            name: None,
            chara: None,
            con_type: None,
            medal_cnt: Vec::new(),
            account: Some(AccountResponse::from(profile)),
            eaappli: Some(EaAppliResponse {
                relation: profile.eaappli_relation,
            }),
            info: Some(InfoResponse {
                ep: profile.ep,
                estatus: Some(profile.estatus),
            }),
            config: Some(profile.config.clone().into()),
            option: Some(profile.option.clone().into()),
            item: profile.items.iter().cloned().map(Into::into).collect(),
            chara_param: profile.characters.iter().cloned().map(Into::into).collect(),
            chara_param_old: Vec::new(),
            customize: Some(CustomizeResponse::from(profile)),
            netvs: Some(NetvsResponse::from(&profile.netvs)),
            course_data: Vec::new(),
            ex_info: profile.extra.iter().cloned().map(Into::into).collect(),
            event_p29: (!profile.event.baskets.is_empty()
                || profile.event.basket_id.is_some()
                || profile.event.ensta_checked.is_some())
            .then(|| EventResponse::from(&profile.event)),
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "medal_cnt")]
pub(super) struct MedalCount {
    clear_type: i8,
    cnt: i16,
}

#[derive(Kbin)]
#[kbin(node = "account")]
pub(super) struct AccountResponse {
    pub(super) g_pm_id: String,
    pub(super) name: String,
    tutorial: i16,
    read_news: i16,
    is_conv: i8,
    active_fr_num: u8,
    total_play_cnt: i16,
    #[kbin(array)]
    pub(super) latest_music: Vec<i16>,
    #[kbin(array)]
    nice: Vec<i16>,
    #[kbin(array)]
    favorite_chara: Vec<i16>,
    popn_class: Option<i8>,
    power_point: i32,
    #[kbin(array)]
    power_point_list: Vec<i32>,
    option_tuto: Option<bool>,
    sc_news_no: Option<i32>,
    read_policy: Option<i16>,
    language: Option<i8>,
}

impl From<&PlayerProfile> for AccountResponse {
    fn from(profile: &PlayerProfile) -> Self {
        Self {
            g_pm_id: profile.g_pm_id.clone(),
            name: profile.name.clone(),
            tutorial: profile.tutorial,
            read_news: profile.read_news,
            is_conv: 0,
            active_fr_num: 0,
            total_play_cnt: profile.total_play_cnt,
            latest_music: normalized(&profile.latest_music, LATEST_MUSIC_HISTORY_COUNT, -1),
            nice: normalized(&profile.nice, NICE_HISTORY_COUNT, -1),
            favorite_chara: normalized(
                &profile.favorite_chara,
                FAVORITE_CHARACTER_HISTORY_COUNT,
                -1,
            ),
            popn_class: Some(profile.popn_class),
            power_point: profile.power_point,
            power_point_list: normalized(&profile.power_point_list, POWER_POINT_HISTORY_COUNT, -1),
            option_tuto: Some(false),
            sc_news_no: Some(profile.sc_news_no),
            read_policy: Some(profile.read_policy),
            language: Some(profile.language),
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "eaappli")]
pub(super) struct EaAppliResponse {
    relation: i8,
}

#[derive(Kbin)]
#[kbin(node = "info")]
pub(super) struct InfoResponse {
    pub(super) ep: u16,
    estatus: Option<u16>,
}

#[derive(Kbin)]
#[kbin(node = "customize")]
pub(super) struct CustomizeResponse {
    seal_0: u16,
    seal_1: u16,
    seal_2: u16,
    seal_3: u16,
    seal_4: u16,
    seal_5: u16,
    seal_6: u16,
    seat: u16,
    touch_th: u16,
    lane_cover: u16,
    stage_bk: u16,
    pub(super) highlight: u16,
}

impl From<&PlayerProfile> for CustomizeResponse {
    fn from(profile: &PlayerProfile) -> Self {
        let values = normalized(&profile.customize, CUSTOMIZE_FIELD_COUNT, 0);
        Self {
            seal_0: values[0],
            seal_1: values[1],
            seal_2: values[2],
            seal_3: values[3],
            seal_4: values[4],
            seal_5: values[5],
            seal_6: values[6],
            seat: values[7],
            touch_th: values[8],
            lane_cover: values[9],
            stage_bk: values[10],
            highlight: values[11],
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "netvs")]
pub(super) struct NetvsResponse {
    #[kbin(array)]
    record: Vec<i16>,
    #[kbin(repeated)]
    dialog: Vec<String>,
    #[kbin(array)]
    ojama_condition: Vec<i8>,
    #[kbin(array)]
    set_ojama: Vec<i8>,
    #[kbin(array)]
    set_recommend: Vec<i8>,
    netvs_play_cnt: u32,
}

impl From<&NetvsState> for NetvsResponse {
    fn from(value: &NetvsState) -> Self {
        Self {
            record: normalized(&value.record, NETVS_RECORD_COUNT, 0),
            dialog: value
                .dialogs
                .iter()
                .take(NETVS_DIALOG_COUNT)
                .cloned()
                .collect(),
            ojama_condition: normalized(&value.ojama_condition, NETVS_OJAMA_CONDITION_COUNT, 0),
            set_ojama: normalized(&value.set_ojama, NETVS_SET_COUNT, 0),
            set_recommend: normalized(&value.set_recommend, NETVS_SET_COUNT, 0),
            netvs_play_cnt: value.netvs_play_cnt,
        }
    }
}

fn normalized<T: Clone>(values: &[T], length: usize, fill: T) -> Vec<T> {
    let mut values = values[..values.len().min(length)].to_vec();
    values.resize(length, fill);
    values
}
