mod supplement;

use std::{collections::HashSet, fs, path::Path};

use encoding_rs::SHIFT_JIS;
use serde::Deserialize;

use crate::protocol::{NEWS_BODY_BYTES, NEWS_TITLE_BYTES};

pub(crate) use supplement::{
    CharacterSupplement, MusicRecord, MusicSupplement, RankingEntry, RankingInfo, TrackSupplement,
};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct InfoData {
    pub phases: Vec<Phase>,
    pub news: Vec<News>,
    pub ranking_info: Vec<RankingInfo>,
    pub goods: Vec<Goods>,
    pub areas: Vec<Area>,
    pub choco: Vec<Choco>,
    pub festivals: Vec<Festival>,
    pub mission_points: Vec<MissionPoint>,
    pub medals: Vec<Medal>,
    pub character_ranking: Vec<CharacterRanking>,
    #[serde(skip)]
    pub music_catalog: Vec<MusicRecord>,
    #[serde(skip)]
    music_ids: HashSet<i16>,
    pub music_supplements: Vec<MusicSupplement>,
    pub track_supplements: Vec<TrackSupplement>,
    pub character_supplements: Vec<CharacterSupplement>,
    #[serde(skip)]
    pub license_music: Vec<i16>,
    pub license_music_new: Vec<i16>,
}

const SHA256_HEX_LENGTH: usize = 64;
const LICENSE_REQUIRED_MTYPE: u64 = 0x0002_0000;
const FESTIVAL_SLOT_COUNT: usize = 6;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MusicCatalogFile {
    source_sha256: String,
    music: Vec<MusicRecord>,
}

impl InfoData {
    pub(crate) fn load(directory: &Path) -> Result<Self, String> {
        let info_path = directory.join("info.json");
        let music_path = directory.join("music.json");
        let mut data: Self = read_json(&info_path)?;
        let catalog: MusicCatalogFile = read_json(&music_path)?;
        if catalog.source_sha256.len() != SHA256_HEX_LENGTH
            || !catalog
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!(
                "{}: source_sha256 must be {SHA256_HEX_LENGTH} hexadecimal characters",
                music_path.display()
            ));
        }
        data.music_catalog = catalog.music;
        data.license_music = data
            .music_catalog
            .iter()
            .filter(|music| music.mtype & LICENSE_REQUIRED_MTYPE != 0)
            .map(|music| music.music_id)
            .collect();
        data.license_music_new.sort_unstable();
        data.license_music_new.dedup();
        data.validate(&info_path, &music_path)?;
        data.music_ids = data
            .music_catalog
            .iter()
            .map(|music| music.music_id)
            .collect();
        Ok(data)
    }

    pub(crate) fn contains_music(&self, music_id: i16) -> bool {
        self.music_ids.contains(&music_id)
    }

    fn validate(&self, info_path: &Path, music_path: &Path) -> Result<(), String> {
        let mut event_ids = HashSet::new();
        for phase in &self.phases {
            if !event_ids.insert(phase.event_id) {
                return Err(format!(
                    "{}: duplicate phase event_id {}",
                    info_path.display(),
                    phase.event_id
                ));
            }
        }
        for news in &self.news {
            string_bytes(
                "news.title",
                &news.title,
                NEWS_TITLE_BYTES.saturating_sub(1),
                info_path,
            )?;
            string_bytes(
                "news.main",
                &news.main,
                NEWS_BODY_BYTES.saturating_sub(1),
                info_path,
            )?;
        }
        for festival in &self.festivals {
            if festival.gauge.len() != FESTIVAL_SLOT_COUNT
                || festival.music.len() != FESTIVAL_SLOT_COUNT
            {
                return Err(format!(
                    "{}: festival gauge/music arrays must contain {FESTIVAL_SLOT_COUNT} values",
                    info_path.display(),
                ));
            }
        }
        for ranking in &self.ranking_info {
            ranking.validate(info_path)?;
        }
        let mut music_ids = HashSet::new();
        for music in &self.music_catalog {
            music.validate(music_path)?;
            validate_music_id(music.music_id, music_path)?;
            if !music_ids.insert(music.music_id) {
                return Err(format!(
                    "{}: duplicate music_catalog id {}",
                    music_path.display(),
                    music.music_id
                ));
            }
        }
        for music in &self.music_supplements {
            music.validate_supplement(info_path)?;
            validate_music_id(music.music_id, info_path)?;
        }
        for track in &self.track_supplements {
            track.validate(info_path)?;
        }
        for character in &self.character_supplements {
            character.validate(info_path)?;
        }
        for value in &self.license_music_new {
            if !music_ids.contains(value) {
                return Err(format!(
                    "{}: license_music_new references unknown music number {value}",
                    info_path.display()
                ));
            }
        }
        Ok(())
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn validate_music_id(music_id: i16, path: &Path) -> Result<(), String> {
    if music_id >= 0 {
        Ok(())
    } else {
        Err(format!(
            "{}: music number {music_id} must be nonnegative",
            path.display()
        ))
    }
}

pub(super) fn string_bytes(
    name: &str,
    value: &str,
    maximum: usize,
    path: &Path,
) -> Result<(), String> {
    let (encoded, _, had_errors) = SHIFT_JIS.encode(value);
    if had_errors {
        Err(format!(
            "{}: {name} contains text that cannot be encoded as CP932",
            path.display()
        ))
    } else if encoded.len() <= maximum {
        Ok(())
    } else {
        Err(format!(
            "{}: {name} exceeds {maximum} CP932 bytes",
            path.display()
        ))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Phase {
    pub event_id: i16,
    pub phase: i16,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct News {
    pub no: i16,
    #[serde(rename = "type")]
    pub kind: u8,
    pub image_no: i16,
    pub title: String,
    pub main: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Goods {
    pub item_id: i32,
    pub item_type: i16,
    pub price: i32,
    pub goods_type: i16,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Area {
    pub area_id: i16,
    pub end_date: u64,
    pub medal_id: i16,
    pub is_limit: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Choco {
    pub choco_id: i16,
    pub param: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Festival {
    pub fes_id: i16,
    pub gauge_count: i32,
    pub gauge: Vec<i32>,
    pub music: Vec<i32>,
    pub r: i16,
    pub g: i16,
    pub b: i16,
    pub poster: i16,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MissionPoint {
    pub point: i32,
    pub bonus_point: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Medal {
    pub medal_id: i16,
    pub percent: i16,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CharacterRanking {
    pub rank: i32,
    pub kind_id: i32,
    pub point: i32,
    pub month: i32,
}
