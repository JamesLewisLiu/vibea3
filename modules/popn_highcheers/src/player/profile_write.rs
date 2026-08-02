use vibea3::Kbin;

use super::event::EventWrite;
use super::state::{CharacterData, ConfigData, ExtraData, ItemData, NetvsWrite, OptionData};
use crate::{
    database::{EventBasket, EventState, ProfilePatch},
    info::InfoData,
    protocol::{
        FAVORITE_CHARACTER_HISTORY_COUNT, LATEST_MUSIC_HISTORY_COUNT, NICE_HISTORY_COUNT,
        POWER_POINT_HISTORY_COUNT,
    },
};

#[derive(Kbin)]
#[kbin(node = "account")]
pub(super) struct AccountWrite {
    pub play_id: Option<i32>,
    pub start_type: Option<i8>,
    pub tutorial: Option<i16>,
    pub read_news: Option<i16>,
    #[kbin(array)]
    pub latest_music: Option<Vec<i16>>,
    #[kbin(array)]
    pub nice: Option<Vec<i16>>,
    #[kbin(array)]
    pub favorite_chara: Option<Vec<i16>>,
    pub popn_class: Option<i8>,
    pub power_point: Option<i32>,
    #[kbin(array)]
    pub power_point_list: Option<Vec<i32>>,
    pub sc_news_no: Option<i32>,
    pub read_policy: Option<i16>,
    pub language: Option<i8>,
}

#[derive(Kbin)]
#[kbin(node = "info")]
pub(super) struct InfoWrite {
    pub ep: Option<u16>,
    pub estatus: Option<u16>,
}

#[derive(Kbin)]
#[kbin(node = "customize")]
pub(super) struct CustomizeWrite {
    pub seal_0: Option<u16>,
    pub seal_1: Option<u16>,
    pub seal_2: Option<u16>,
    pub seal_3: Option<u16>,
    pub seal_4: Option<u16>,
    pub seal_5: Option<u16>,
    pub seal_6: Option<u16>,
    pub seat: Option<u16>,
    pub touch_th: Option<u16>,
    pub lane_cover: Option<u16>,
    pub stage_bk: Option<u16>,
    pub highlight: Option<u16>,
}

pub(super) struct ProfileWrite {
    pub account: Option<AccountWrite>,
    pub info: Option<InfoWrite>,
    pub customize: Option<CustomizeWrite>,
    pub option: Option<OptionData>,
    pub config: Option<ConfigData>,
    pub items: Vec<ItemData>,
    pub characters: Vec<CharacterData>,
    pub extra: Vec<ExtraData>,
    pub netvs: Option<NetvsWrite>,
    pub event: Option<EventWrite>,
}

pub(super) fn patch(input: ProfileWrite, catalog: &InfoData) -> ProfilePatch {
    let account = input.account;
    let info = input.info.unwrap_or(InfoWrite {
        ep: None,
        estatus: None,
    });
    ProfilePatch {
        tutorial: account.as_ref().and_then(|value| value.tutorial),
        read_news: account.as_ref().and_then(|value| value.read_news),
        latest_music: account
            .as_ref()
            .and_then(|value| value.latest_music.clone())
            .map(|value| normalize(value, LATEST_MUSIC_HISTORY_COUNT, -1)),
        nice: account
            .as_ref()
            .and_then(|value| value.nice.clone())
            .map(|value| normalize(value, NICE_HISTORY_COUNT, -1)),
        favorite_chara: account
            .as_ref()
            .and_then(|value| value.favorite_chara.clone())
            .map(|value| normalize(value, FAVORITE_CHARACTER_HISTORY_COUNT, -1)),
        popn_class: account.as_ref().and_then(|value| value.popn_class),
        power_point: account.as_ref().and_then(|value| value.power_point),
        power_point_list: account
            .as_ref()
            .and_then(|value| value.power_point_list.clone())
            .map(|value| normalize(value, POWER_POINT_HISTORY_COUNT, -1)),
        sc_news_no: account.as_ref().and_then(|value| value.sc_news_no),
        read_policy: account.as_ref().and_then(|value| value.read_policy),
        language: account.as_ref().and_then(|value| value.language),
        ep: info.ep,
        estatus: info.estatus,
        customize: input.customize.map(|value| {
            vec![
                value.seal_0.unwrap_or_default(),
                value.seal_1.unwrap_or_default(),
                value.seal_2.unwrap_or_default(),
                value.seal_3.unwrap_or_default(),
                value.seal_4.unwrap_or_default(),
                value.seal_5.unwrap_or_default(),
                value.seal_6.unwrap_or_default(),
                value.seat.unwrap_or_default(),
                value.touch_th.unwrap_or_default(),
                value.lane_cover.unwrap_or_default(),
                value.stage_bk.unwrap_or_default(),
                value.highlight.unwrap_or_default(),
            ]
        }),
        option: input.option.map(Into::into),
        config: input.config.map(Into::into),
        items: Some(input.items.into_iter().map(Into::into).collect()),
        characters: Some(input.characters.into_iter().map(Into::into).collect()),
        extra: Some(
            input
                .extra
                .into_iter()
                .filter(|value| catalog.contains_music(value.music_num))
                .map(Into::into)
                .collect(),
        ),
        netvs: input.netvs.map(Into::into),
        event: input.event.map(|value| EventState {
            basket_id: value.basket_id,
            baskets: value
                .basket
                .into_iter()
                .filter(|basket| basket.id.is_positive())
                .map(|basket| EventBasket {
                    id: basket.id,
                    point: basket.point,
                    is_cleared: basket.is_cleared,
                })
                .collect(),
            ensta_checked: value.ensta.map(|ensta| ensta.checked),
            ensta_serial_code: None,
        }),
    }
}

fn normalize<T: Clone>(mut values: Vec<T>, length: usize, fill: T) -> Vec<T> {
    values.truncate(length);
    values.resize(length, fill);
    values
}
