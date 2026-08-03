use std::{fs, path::PathBuf};

use serde::Deserialize;
use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use crate::State;
use wire::{
    AkanamePart, AppealCardInfo, CatalogInfo, EventInfo, MusicInfo, MusicLimitedInfo,
    SkillCourseInfo,
};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CommonData {
    pub release_code: String,
    pub lounge_interval: u32,
    pub skill_courses: Vec<SkillCourseInfo>,
    pub events: Vec<EventInfo>,
    pub catalog: Vec<CatalogInfo>,
    pub music_limited: Vec<MusicLimitedInfo>,
    pub music: Vec<MusicInfo>,
    pub appeal_cards: Vec<AppealCardInfo>,
    pub akaname_parts: Vec<AkanamePart>,
}

#[derive(Deserialize)]
struct InfoFile {
    release_code: String,
    lounge_interval: u32,
    skill_courses: Vec<SkillCourseInfo>,
    events: Vec<EventInfo>,
    catalog: Vec<CatalogInfo>,
    music_limited: Vec<MusicLimitedInfo>,
}

#[derive(Deserialize)]
struct CatalogFile {
    source_sha256: SourceHashes,
    music: Vec<MusicInfo>,
    appeal_cards: Vec<AppealCardInfo>,
    akaname_parts: Vec<AkanamePart>,
}

#[derive(Deserialize)]
struct SourceHashes {
    #[serde(rename = "music_db.xml")]
    music: String,
    #[serde(rename = "appeal_card.xml")]
    appeal: String,
    #[serde(rename = "akaname_parts.xml")]
    akaname: String,
}

const SHA256_HEX_LENGTH: usize = 64;

pub(crate) fn load() -> Result<CommonData, String> {
    let directory = std::env::var_os("SDVX_NABULA_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("VIBEA3_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data"))
                .join("sdvx_nabula")
        });
    let info: InfoFile = read_json(directory.join("info.json"))?;
    let catalog: CatalogFile = read_json(directory.join("catalog.json"))?;
    for (name, hash) in [
        ("music_db.xml", &catalog.source_sha256.music),
        ("appeal_card.xml", &catalog.source_sha256.appeal),
        ("akaname_parts.xml", &catalog.source_sha256.akaname),
    ] {
        if hash.len() != SHA256_HEX_LENGTH || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("{name} source hash is not a SHA-256 digest"));
        }
    }
    Ok(CommonData {
        release_code: info.release_code,
        lounge_interval: info.lounge_interval,
        skill_courses: info.skill_courses,
        events: info.events,
        catalog: info.catalog,
        music_limited: info.music_limited,
        music: catalog.music,
        appeal_cards: catalog.appeal_cards,
        akaname_parts: catalog.akaname_parts,
    })
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T, String> {
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid {}: {error}", path.display()))
}

mod wire;
pub(crate) use wire::EmptyRequest;
