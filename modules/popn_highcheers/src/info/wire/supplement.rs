use vibea3::Kbin;

use super::data;

#[derive(Kbin)]
#[kbin(node = "ranking_info")]
pub(crate) struct RankingInfo {
    course_id: i16,
    start_date: u64,
    end_date: u64,
    music_id: i32,
    #[kbin(repeated)]
    loc_ranking_e: Vec<RankingEntryE>,
    #[kbin(repeated)]
    loc_ranking_n: Vec<RankingEntryN>,
    #[kbin(repeated)]
    loc_ranking_h: Vec<RankingEntryH>,
    #[kbin(repeated)]
    loc_ranking_ex: Vec<RankingEntryEx>,
}

impl From<&data::RankingInfo> for RankingInfo {
    fn from(value: &data::RankingInfo) -> Self {
        Self {
            course_id: value.course_id,
            start_date: value.start_date,
            end_date: value.end_date,
            music_id: value.music_id,
            loc_ranking_e: value
                .loc_ranking_e
                .iter()
                .map(RankingEntryE::from)
                .collect(),
            loc_ranking_n: value
                .loc_ranking_n
                .iter()
                .map(RankingEntryN::from)
                .collect(),
            loc_ranking_h: value
                .loc_ranking_h
                .iter()
                .map(RankingEntryH::from)
                .collect(),
            loc_ranking_ex: value
                .loc_ranking_ex
                .iter()
                .map(RankingEntryEx::from)
                .collect(),
        }
    }
}

macro_rules! ranking_entry {
    ($name:ident, $node:literal) => {
        #[derive(Kbin)]
        #[kbin(node = $node)]
        struct $name {
            rank: i16,
            name: String,
            chara_num: i16,
            total_score: i32,
            clear_type: u8,
            clear_rank: u8,
        }
        impl From<&data::RankingEntry> for $name {
            fn from(value: &data::RankingEntry) -> Self {
                Self {
                    rank: value.rank,
                    name: value.name.clone(),
                    chara_num: value.chara_num,
                    total_score: value.total_score,
                    clear_type: value.clear_type,
                    clear_rank: value.clear_rank,
                }
            }
        }
    };
}

ranking_entry!(RankingEntryE, "loc_ranking_e");
ranking_entry!(RankingEntryN, "loc_ranking_n");
ranking_entry!(RankingEntryH, "loc_ranking_h");
ranking_entry!(RankingEntryEx, "loc_ranking_ex");

#[derive(Kbin)]
#[kbin(node = "musicsub")]
pub(crate) struct MusicSupplement {
    music_id: i16,
    name_sort: String,
    title_sort: String,
    artist_sort: String,
    name: String,
    title: String,
    artist: String,
    chr: i16,
    chr2: i16,
    mtype: u64,
    ac_ver: i32,
    cs_ver: i32,
    bm_from_u32: u32,
    #[kbin(array)]
    lv: Vec<u8>,
    #[kbin(array)]
    track: Vec<u16>,
    sp_hariai: String,
    sp_x: i16,
    sp_y: i16,
    #[kbin(array)]
    tag_list: Vec<u16>,
    #[kbin(array)]
    bpm_min: Vec<i16>,
    #[kbin(array)]
    bpm_max: Vec<i16>,
    #[kbin(array)]
    long: Vec<i8>,
}

impl From<&data::MusicSupplement> for MusicSupplement {
    fn from(v: &data::MusicSupplement) -> Self {
        Self {
            music_id: v.music_id,
            name_sort: v.name_sort.clone(),
            title_sort: v.title_sort.clone(),
            artist_sort: v.artist_sort.clone(),
            name: v.name.clone(),
            title: v.title.clone(),
            artist: v.artist.clone(),
            chr: v.chr,
            chr2: v.chr2,
            mtype: v.mtype,
            ac_ver: v.ac_ver,
            cs_ver: v.cs_ver,
            bm_from_u32: v.bm_from_u32,
            lv: v.lv.clone(),
            track: v.track.clone(),
            sp_hariai: v.sp_hariai.clone(),
            sp_x: v.sp_x,
            sp_y: v.sp_y,
            tag_list: v.tag_list.clone(),
            bpm_min: v.bpm_min.clone(),
            bpm_max: v.bpm_max.clone(),
            long: v.long.clone(),
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "tracksub")]
pub(crate) struct TrackSupplement {
    track_id: i16,
    series: String,
    track: String,
    element: i32,
    backtrack: i32,
    preview: i32,
    master: i32,
    updatever: i32,
    easylanedata: u16,
}

impl From<&data::TrackSupplement> for TrackSupplement {
    fn from(v: &data::TrackSupplement) -> Self {
        Self {
            track_id: v.track_id,
            series: v.series.clone(),
            track: v.track.clone(),
            element: v.element,
            backtrack: v.backtrack,
            preview: v.preview,
            master: v.master,
            updatever: v.updatever,
            easylanedata: v.easylanedata,
        }
    }
}

#[derive(Kbin)]
#[kbin(node = "charasub")]
pub(crate) struct CharacterSupplement {
    chara_id: i16,
    filename: String,
    #[kbin(rename = "type")]
    kind: u32,
    parent: String,
    name: String,
    icon: String,
    icon_1p: String,
    icon_2p: String,
    #[kbin(array)]
    loc_btl_bmp: Vec<i16>,
    disptype: i16,
    chr_value: i16,
    value_rank: i16,
    sort: String,
    disp: String,
    update_ver: i16,
    hariainame: String,
    catchcopy: String,
    version_no: u32,
    hariai_type: i16,
}

impl From<&data::CharacterSupplement> for CharacterSupplement {
    fn from(v: &data::CharacterSupplement) -> Self {
        Self {
            chara_id: v.chara_id,
            filename: v.filename.clone(),
            kind: v.kind,
            parent: v.parent.clone(),
            name: v.name.clone(),
            icon: v.icon.clone(),
            icon_1p: v.icon_1p.clone(),
            icon_2p: v.icon_2p.clone(),
            loc_btl_bmp: v.loc_btl_bmp.clone(),
            disptype: v.disptype,
            chr_value: v.chr_value,
            value_rank: v.value_rank,
            sort: v.sort.clone(),
            disp: v.disp.clone(),
            update_ver: v.update_ver,
            hariainame: v.hariainame.clone(),
            catchcopy: v.catchcopy.clone(),
            version_no: v.version_no,
            hariai_type: v.hariai_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use vibea3::{DecodeOptions, EncodeOptions, decode_kbin, encode_kbin, encode_xml};

    use super::*;

    #[test]
    fn supplement_psmap_character_buffers_are_wire_strings() {
        let music = MusicSupplement::from(&data::MusicSupplement {
            music_id: 1,
            name_sort: "NAME_SORT".into(),
            title_sort: "TITLE_SORT".into(),
            artist_sort: "ARTIST_SORT".into(),
            name: "NAME".into(),
            title: "TITLE".into(),
            artist: "ARTIST".into(),
            chr: 1,
            chr2: 2,
            mtype: 0,
            ac_ver: 0,
            cs_ver: 0,
            bm_from_u32: 0,
            lv: vec![0; 7],
            track: vec![0; 7],
            sp_hariai: "HARIAI".into(),
            sp_x: 0,
            sp_y: 0,
            tag_list: vec![0; 32],
            bpm_min: vec![0; 7],
            bpm_max: vec![0; 7],
            long: vec![0; 7],
        });
        let xml = String::from_utf8(encode_xml(&music, EncodeOptions::default()).unwrap()).unwrap();
        for (field, value) in [
            ("name_sort", "NAME_SORT"),
            ("title", "TITLE"),
            ("artist", "ARTIST"),
            ("sp_hariai", "HARIAI"),
        ] {
            assert!(xml.contains(&format!("<{field} __type=\"str\">{value}</{field}>")));
        }
        assert!(!xml.contains("__type=\"bin\""));
        let kbin = encode_kbin(&music, EncodeOptions::default()).unwrap();
        let decoded = decode_kbin::<MusicSupplement>(&kbin, DecodeOptions::default()).unwrap();
        assert_eq!(decoded.title, "TITLE");
        assert_eq!(decoded.sp_hariai, "HARIAI");

        let character = CharacterSupplement::from(&data::CharacterSupplement {
            chara_id: 1,
            filename: "FILE".into(),
            kind: 0,
            parent: "PARENT".into(),
            name: "NAME".into(),
            icon: "ICON".into(),
            icon_1p: "ICON1".into(),
            icon_2p: "ICON2".into(),
            loc_btl_bmp: vec![0; 2],
            disptype: 0,
            chr_value: 0,
            value_rank: 0,
            sort: "SORT".into(),
            disp: "DISP".into(),
            update_ver: 0,
            hariainame: "HARIAI".into(),
            catchcopy: "CATCH".into(),
            version_no: 0,
            hariai_type: 0,
        });
        let xml =
            String::from_utf8(encode_xml(&character, EncodeOptions::default()).unwrap()).unwrap();
        assert!(xml.contains("<filename __type=\"str\">FILE</filename>"));
        assert!(xml.contains("<catchcopy __type=\"str\">CATCH</catchcopy>"));
        assert!(!xml.contains("__type=\"bin\""));
        let kbin = encode_kbin(&character, EncodeOptions::default()).unwrap();
        let decoded = decode_kbin::<CharacterSupplement>(&kbin, DecodeOptions::default()).unwrap();
        assert_eq!(decoded.filename, "FILE");
        assert_eq!(decoded.catchcopy, "CATCH");
    }
}
