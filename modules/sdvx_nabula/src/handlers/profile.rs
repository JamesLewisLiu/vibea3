use bson::{DateTime, doc};
use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use crate::{
    State,
    database::{
        MusicScore, PlayerItem, PlayerParam, PlayerProfile, PlayerSetting, RadarElements,
        StoryProgress,
    },
    protocol::{
        MUSIC_PARAM_BUTTON_RATE, MUSIC_PARAM_LONG_RATE, MUSIC_PARAM_MUSIC_ID,
        MUSIC_PARAM_MUSIC_TYPE, MUSIC_PARAM_PRIMARY_CLEAR_COUNT, MUSIC_PARAM_PRIMARY_CLEAR_TYPE,
        MUSIC_PARAM_PRIMARY_EXSCORE, MUSIC_PARAM_PRIMARY_GRADE, MUSIC_PARAM_PRIMARY_MAX_CHAIN,
        MUSIC_PARAM_PRIMARY_PLAY_COUNT, MUSIC_PARAM_PRIMARY_PUC_COUNT, MUSIC_PARAM_PRIMARY_SCORE,
        MUSIC_PARAM_PRIMARY_UC_COUNT, MUSIC_PARAM_VOL_RATE, MUSIC_PARAM_VOLFORCE,
        MUSIC_RECORD_PARAM_COUNT, PARAMETER_VALUE_COUNT, RIVAL_MUSIC_PARAM_COUNT,
    },
};

use super::{database_error, invalid_session};

#[derive(Kbin)]
#[kbin(node = "game")]
struct NewRequest {
    dataid: String,
    refid: String,
    cardno: Option<String>,
    name: String,
    locid: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct LoadRequest {
    dataid: String,
    cardid: Option<String>,
    refid: String,
    cardno: Option<String>,
    locid: Option<String>,
    cardmng: Option<CardManagement>,
}

#[derive(Kbin)]
#[kbin(node = "cardmng")]
struct CardManagement {
    #[kbin(repeated)]
    item: Vec<CardManagementItem>,
}

#[derive(Kbin)]
#[kbin(node = "item")]
struct CardManagementItem {
    mcode: String,
    regtime: i32,
    lasttime: i32,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct RefIdRequest {
    refid: String,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct RefIdUnderscoreRequest {
    ref_id: String,
    mix_code: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "game")]
struct EmptyResponse {}

mod wire;
