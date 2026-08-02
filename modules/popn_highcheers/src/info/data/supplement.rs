use std::path::Path;

use serde::Deserialize;

use super::string_bytes;
use crate::protocol::{
    AUXILIARY_TEXT_BYTES, CHARACTER_BATTLE_BITMAP_COUNT, FIXED_PLAYER_TEXT_PAYLOAD_BYTES,
    MUSIC_TAG_COUNT, MUSIC_TEXT_BYTES, PLAYABLE_SHEET_COUNT,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RankingInfo {
    pub course_id: i16,
    pub start_date: u64,
    pub end_date: u64,
    pub music_id: i32,
    #[serde(default)]
    pub loc_ranking_e: Vec<RankingEntry>,
    #[serde(default)]
    pub loc_ranking_n: Vec<RankingEntry>,
    #[serde(default)]
    pub loc_ranking_h: Vec<RankingEntry>,
    #[serde(default)]
    pub loc_ranking_ex: Vec<RankingEntry>,
}

impl RankingInfo {
    pub(super) fn validate(&self, path: &Path) -> Result<(), String> {
        for entries in [
            &self.loc_ranking_e,
            &self.loc_ranking_n,
            &self.loc_ranking_h,
            &self.loc_ranking_ex,
        ] {
            for entry in entries {
                string_bytes(
                    "ranking.name",
                    &entry.name,
                    FIXED_PLAYER_TEXT_PAYLOAD_BYTES,
                    path,
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RankingEntry {
    pub rank: i16,
    pub name: String,
    pub chara_num: i16,
    pub total_score: i32,
    pub clear_type: u8,
    pub clear_rank: u8,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MusicRecord {
    pub music_id: i16,
    pub name_sort: String,
    pub title_sort: String,
    pub artist_sort: String,
    pub name: String,
    pub title: String,
    pub artist: String,
    pub chr: i16,
    pub chr2: i16,
    pub mtype: u64,
    pub ac_ver: i32,
    pub cs_ver: i32,
    pub bm_from_u32: u32,
    pub lv: Vec<u8>,
    pub track: Vec<u16>,
    pub sp_hariai: String,
    pub sp_x: i16,
    pub sp_y: i16,
    pub tag_list: Vec<u16>,
    pub bpm_min: Vec<i16>,
    pub bpm_max: Vec<i16>,
    pub long: Vec<i8>,
}

pub(crate) type MusicSupplement = MusicRecord;

impl MusicRecord {
    pub(super) fn validate(&self, path: &Path) -> Result<(), String> {
        exact("lv", self.lv.len(), PLAYABLE_SHEET_COUNT, path)?;
        exact("track", self.track.len(), PLAYABLE_SHEET_COUNT, path)?;
        exact("tag_list", self.tag_list.len(), MUSIC_TAG_COUNT, path)?;
        exact("bpm_min", self.bpm_min.len(), PLAYABLE_SHEET_COUNT, path)?;
        exact("bpm_max", self.bpm_max.len(), PLAYABLE_SHEET_COUNT, path)?;
        exact("long", self.long.len(), PLAYABLE_SHEET_COUNT, path)
    }

    pub(super) fn validate_supplement(&self, path: &Path) -> Result<(), String> {
        self.validate(path)?;
        for (name, value) in [
            ("name_sort", &self.name_sort),
            ("title_sort", &self.title_sort),
            ("artist_sort", &self.artist_sort),
            ("name", &self.name),
            ("title", &self.title),
            ("artist", &self.artist),
        ] {
            string_bytes(name, value, MUSIC_TEXT_BYTES.saturating_sub(1), path)?;
        }
        string_bytes(
            "sp_hariai",
            &self.sp_hariai,
            AUXILIARY_TEXT_BYTES.saturating_sub(1),
            path,
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TrackSupplement {
    pub track_id: i16,
    pub series: String,
    pub track: String,
    pub element: i32,
    pub backtrack: i32,
    pub preview: i32,
    pub master: i32,
    pub updatever: i32,
    pub easylanedata: u16,
}

impl TrackSupplement {
    pub(super) fn validate(&self, path: &Path) -> Result<(), String> {
        string_bytes(
            "track.series",
            &self.series,
            AUXILIARY_TEXT_BYTES.saturating_sub(1),
            path,
        )?;
        string_bytes(
            "track.track",
            &self.track,
            AUXILIARY_TEXT_BYTES.saturating_sub(1),
            path,
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CharacterSupplement {
    pub chara_id: i16,
    pub filename: String,
    #[serde(rename = "type")]
    pub kind: u32,
    pub parent: String,
    pub name: String,
    pub icon: String,
    pub icon_1p: String,
    pub icon_2p: String,
    pub loc_btl_bmp: Vec<i16>,
    pub disptype: i16,
    pub chr_value: i16,
    pub value_rank: i16,
    pub sort: String,
    pub disp: String,
    pub update_ver: i16,
    pub hariainame: String,
    pub catchcopy: String,
    pub version_no: u32,
    pub hariai_type: i16,
}

impl CharacterSupplement {
    pub(super) fn validate(&self, path: &Path) -> Result<(), String> {
        for (name, value) in [
            ("filename", &self.filename),
            ("parent", &self.parent),
            ("name", &self.name),
            ("icon", &self.icon),
            ("icon_1p", &self.icon_1p),
            ("icon_2p", &self.icon_2p),
            ("sort", &self.sort),
            ("disp", &self.disp),
            ("hariainame", &self.hariainame),
            ("catchcopy", &self.catchcopy),
        ] {
            string_bytes(name, value, AUXILIARY_TEXT_BYTES.saturating_sub(1), path)?;
        }
        exact(
            "loc_btl_bmp",
            self.loc_btl_bmp.len(),
            CHARACTER_BATTLE_BITMAP_COUNT,
            path,
        )
    }
}

fn exact(name: &str, actual: usize, expected: usize, path: &Path) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{}: {name} has {actual} entries; expected {expected}",
            path.display()
        ))
    }
}
