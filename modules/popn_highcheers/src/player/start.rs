use vibea3::Kbin;

use crate::{
    database::UsageSnapshot,
    info::{
        InfoData,
        wire::{
            Area, CharacterRanking, Choco, CommonResponse, Festival, Goods, Medal, MissionPoint,
            News, Phase, PopularCharacter, PopularMusic, Recommend,
            supplement::{CharacterSupplement, MusicSupplement, RankingInfo, TrackSupplement},
        },
    },
};

#[derive(Kbin)]
#[kbin(node = "player")]
pub(super) struct StartResponse {
    pub play_id: i32,
    #[kbin(repeated)]
    phase: Vec<Phase>,
    #[kbin(repeated)]
    news: Vec<News>,
    #[kbin(repeated)]
    ranking_info: Vec<RankingInfo>,
    #[kbin(repeated)]
    goods: Vec<Goods>,
    #[kbin(repeated, rename = "area")]
    areas: Vec<Area>,
    #[kbin(repeated)]
    choco: Vec<Choco>,
    #[kbin(repeated, rename = "fes")]
    festivals: Vec<Festival>,
    #[kbin(repeated, rename = "popular")]
    popular_characters: Vec<PopularCharacter>,
    #[kbin(repeated, rename = "popular_music")]
    popular_music: Vec<PopularMusic>,
    recommend: Option<Recommend>,
    #[kbin(repeated, rename = "mission_point")]
    mission_points: Vec<MissionPoint>,
    #[kbin(repeated, rename = "medal")]
    medals: Vec<Medal>,
    #[kbin(repeated, rename = "chara_ranking")]
    character_ranking: Vec<CharacterRanking>,
    #[kbin(repeated, rename = "musicsub")]
    music_supplements: Vec<MusicSupplement>,
    #[kbin(repeated, rename = "tracksub")]
    track_supplements: Vec<TrackSupplement>,
    #[kbin(repeated, rename = "charasub")]
    character_supplements: Vec<CharacterSupplement>,
    #[kbin(array)]
    license_music: Option<Vec<i16>>,
    #[kbin(array)]
    license_music_new: Option<Vec<i16>>,
}

impl StartResponse {
    pub(super) fn new(play_id: i32, data: &InfoData, usage: &UsageSnapshot) -> Self {
        let common = CommonResponse::new(data, usage);
        Self {
            play_id,
            phase: common.phase,
            news: common.news,
            ranking_info: common.ranking_info,
            goods: common.goods,
            areas: common.areas,
            choco: common.choco,
            festivals: common.festivals,
            popular_characters: common.popular_characters,
            popular_music: common.popular_music,
            recommend: common.recommend,
            mission_points: common.mission_points,
            medals: common.medals,
            character_ranking: common.character_ranking,
            music_supplements: common.music_supplements,
            track_supplements: common.track_supplements,
            character_supplements: common.character_supplements,
            license_music: common.license_music,
            license_music_new: common.license_music_new,
        }
    }
}
