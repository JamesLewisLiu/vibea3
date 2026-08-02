pub(crate) mod supplement;

use vibea3::Kbin;

use super::data;
use crate::database::UsageSnapshot;
use supplement::{CharacterSupplement, MusicSupplement, RankingInfo, TrackSupplement};

#[derive(Kbin)]
#[kbin(node = "info")]
pub(super) struct CommonRequest {
    #[kbin(attr)]
    pub loc_id: String,
}

#[derive(Kbin)]
#[kbin(node = "info")]
pub(crate) struct CommonResponse {
    #[kbin(repeated)]
    pub(crate) phase: Vec<Phase>,
    #[kbin(repeated)]
    pub(crate) news: Vec<News>,
    #[kbin(repeated)]
    pub(crate) ranking_info: Vec<RankingInfo>,
    #[kbin(repeated)]
    pub(crate) goods: Vec<Goods>,
    #[kbin(repeated, rename = "area")]
    pub(crate) areas: Vec<Area>,
    #[kbin(repeated)]
    pub(crate) choco: Vec<Choco>,
    #[kbin(repeated, rename = "fes")]
    pub(crate) festivals: Vec<Festival>,
    #[kbin(repeated, rename = "popular")]
    pub(crate) popular_characters: Vec<PopularCharacter>,
    #[kbin(repeated, rename = "popular_music")]
    pub(crate) popular_music: Vec<PopularMusic>,
    pub(crate) recommend: Option<Recommend>,
    #[kbin(repeated, rename = "mission_point")]
    pub(crate) mission_points: Vec<MissionPoint>,
    #[kbin(repeated, rename = "medal")]
    pub(crate) medals: Vec<Medal>,
    #[kbin(repeated, rename = "chara_ranking")]
    pub(crate) character_ranking: Vec<CharacterRanking>,
    #[kbin(repeated, rename = "musicsub")]
    pub(crate) music_supplements: Vec<MusicSupplement>,
    #[kbin(repeated, rename = "tracksub")]
    pub(crate) track_supplements: Vec<TrackSupplement>,
    #[kbin(repeated, rename = "charasub")]
    pub(crate) character_supplements: Vec<CharacterSupplement>,
    #[kbin(array)]
    pub(crate) license_music: Option<Vec<i16>>,
    #[kbin(array)]
    pub(crate) license_music_new: Option<Vec<i16>>,
}

impl CommonResponse {
    pub(crate) fn new(value: &data::InfoData, usage: &UsageSnapshot) -> Self {
        Self {
            phase: value.phases.iter().map(Into::into).collect(),
            news: value.news.iter().map(Into::into).collect(),
            ranking_info: value.ranking_info.iter().map(Into::into).collect(),
            goods: value.goods.iter().map(Into::into).collect(),
            areas: value.areas.iter().map(Into::into).collect(),
            choco: value.choco.iter().map(Into::into).collect(),
            festivals: value.festivals.iter().map(Into::into).collect(),
            popular_characters: usage
                .popular_characters
                .iter()
                .enumerate()
                .map(|(rank, &chara_num)| PopularCharacter {
                    rank: (rank + 1) as i16,
                    chara_num,
                })
                .collect(),
            popular_music: usage
                .popular_music
                .iter()
                .map(|&music_num| PopularMusic { music_num })
                .collect(),
            recommend: (!usage.recommend_music.is_empty()).then(|| Recommend {
                music_no: usage.recommend_music.clone(),
            }),
            mission_points: value.mission_points.iter().map(Into::into).collect(),
            medals: value.medals.iter().map(Into::into).collect(),
            character_ranking: value.character_ranking.iter().map(Into::into).collect(),
            music_supplements: value.music_supplements.iter().map(Into::into).collect(),
            track_supplements: value.track_supplements.iter().map(Into::into).collect(),
            character_supplements: value.character_supplements.iter().map(Into::into).collect(),
            license_music: (!value.license_music.is_empty()).then(|| value.license_music.clone()),
            license_music_new: (!value.license_music_new.is_empty())
                .then(|| value.license_music_new.clone()),
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "phase")]
pub(crate) struct Phase {
    event_id: i16,
    phase: i16,
}

impl From<&data::Phase> for Phase {
    fn from(value: &data::Phase) -> Self {
        Self {
            event_id: value.event_id,
            phase: value.phase,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "news")]
pub(crate) struct News {
    no: i16,
    #[kbin(rename = "type")]
    kind: u8,
    image_no: i16,
    title: String,
    main: String,
}

impl From<&data::News> for News {
    fn from(value: &data::News) -> Self {
        Self {
            no: value.no,
            kind: value.kind,
            image_no: value.image_no,
            title: value.title.clone(),
            main: value.main.clone(),
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "goods")]
pub(crate) struct Goods {
    item_id: i32,
    item_type: i16,
    price: i32,
    goods_type: i16,
}

impl From<&data::Goods> for Goods {
    fn from(value: &data::Goods) -> Self {
        Self {
            item_id: value.item_id,
            item_type: value.item_type,
            price: value.price,
            goods_type: value.goods_type,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "area")]
pub(crate) struct Area {
    area_id: i16,
    end_date: u64,
    medal_id: i16,
    is_limit: bool,
}

impl From<&data::Area> for Area {
    fn from(value: &data::Area) -> Self {
        Self {
            area_id: value.area_id,
            end_date: value.end_date,
            medal_id: value.medal_id,
            is_limit: value.is_limit,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "choco")]
pub(crate) struct Choco {
    choco_id: i16,
    param: i32,
}

impl From<&data::Choco> for Choco {
    fn from(value: &data::Choco) -> Self {
        Self {
            choco_id: value.choco_id,
            param: value.param,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "fes")]
pub(crate) struct Festival {
    fes_id: i16,
    gauge_count: i32,
    #[kbin(array)]
    gauge: Vec<i32>,
    #[kbin(array)]
    music: Vec<i32>,
    r: i16,
    g: i16,
    b: i16,
    poster: i16,
}

impl From<&data::Festival> for Festival {
    fn from(value: &data::Festival) -> Self {
        Self {
            fes_id: value.fes_id,
            gauge_count: value.gauge_count,
            gauge: value.gauge.clone(),
            music: value.music.clone(),
            r: value.r,
            g: value.g,
            b: value.b,
            poster: value.poster,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "popular")]
pub(crate) struct PopularCharacter {
    rank: i16,
    chara_num: i16,
}

#[derive(Kbin)]
#[kbin(node = "popular_music")]
pub(crate) struct PopularMusic {
    music_num: i16,
}

#[derive(Kbin)]
#[kbin(node = "recommend")]
pub(crate) struct Recommend {
    #[kbin(array)]
    music_no: Vec<i32>,
}

#[derive(Kbin)]
#[kbin(node = "mission_point")]
pub(crate) struct MissionPoint {
    point: i32,
    bonus_point: i32,
}

impl From<&data::MissionPoint> for MissionPoint {
    fn from(value: &data::MissionPoint) -> Self {
        Self {
            point: value.point,
            bonus_point: value.bonus_point,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "medal")]
pub(crate) struct Medal {
    medal_id: i16,
    percent: i16,
}

impl From<&data::Medal> for Medal {
    fn from(value: &data::Medal) -> Self {
        Self {
            medal_id: value.medal_id,
            percent: value.percent,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "chara_ranking")]
pub(crate) struct CharacterRanking {
    rank: i32,
    kind_id: i32,
    point: i32,
    month: i32,
}

impl From<&data::CharacterRanking> for CharacterRanking {
    fn from(value: &data::CharacterRanking) -> Self {
        Self {
            rank: value.rank,
            kind_id: value.kind_id,
            point: value.point,
            month: value.month,
        }
    }
}

#[cfg(test)]
mod tests {
    use vibea3::{DecodeOptions, EncodeOptions, decode_kbin, encode_kbin, encode_xml};

    use super::*;

    #[test]
    fn news_psmap_character_buffers_are_wire_strings() {
        let news = News::from(&data::News {
            no: 1,
            kind: 2,
            image_no: 3,
            title: "TITLE".into(),
            main: "BODY".into(),
        });
        let xml = String::from_utf8(encode_xml(&news, EncodeOptions::default()).unwrap()).unwrap();
        assert!(xml.contains("<title __type=\"str\">TITLE</title>"));
        assert!(xml.contains("<main __type=\"str\">BODY</main>"));
        assert!(!xml.contains("__type=\"bin\""));

        let kbin = encode_kbin(&news, EncodeOptions::default()).unwrap();
        let decoded = decode_kbin::<News>(&kbin, DecodeOptions::default()).unwrap();
        assert_eq!(decoded.title, "TITLE");
        assert_eq!(decoded.main, "BODY");
    }
}
