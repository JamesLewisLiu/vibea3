use vibea3::{
    DecodeOptions, EncodeOptions, Kbin, NameMode, decode_kbin, decode_xml, encode_kbin, encode_xml,
};

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "mdb")]
struct MusicDb {
    #[kbin(repeated)]
    music: Vec<Music>,
}

#[test]
fn decode_external_musicdb_kbin_when_requested() {
    let Ok(input) = std::env::var("VIBEA3_KBIN_SAMPLE") else {
        eprintln!("skipping external kbin sample; VIBEA3_KBIN_SAMPLE is unset");
        return;
    };
    let options = DecodeOptions {
        max_nodes: 500_000,
        ..DecodeOptions::default()
    };
    let db = decode_kbin::<MusicDb>(&std::fs::read(input).unwrap(), options).unwrap();
    assert_eq!(db.music.len(), 2_244);
    assert_eq!(db.music[0].id, 1);
    assert_eq!(db.music.last().unwrap().id, 2_362);

    if let Ok(xml) = std::env::var("VIBEA3_XML_ROUNDTRIP") {
        let round_trip = decode_xml::<MusicDb>(&std::fs::read(xml).unwrap(), options).unwrap();
        assert_eq!(round_trip, db);
    }
    if let Ok(kbin) = std::env::var("VIBEA3_KBIN_ROUNDTRIP") {
        let round_trip = decode_kbin::<MusicDb>(&std::fs::read(kbin).unwrap(), options).unwrap();
        assert_eq!(round_trip, db);
    }

    if let Ok(output) = std::env::var("VIBEA3_KBIN_REENCODE") {
        let bytes = encode_kbin(
            &db,
            EncodeOptions {
                encoding: 0x80,
                names: NameMode::Full,
            },
        )
        .unwrap();
        std::fs::write(output, bytes).unwrap();
    }
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "music")]
struct Music {
    #[kbin(attr)]
    id: u32,
    info: Info,
    difficulty: Difficulty,
    #[kbin(repeated)]
    tag: Vec<Tag>,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "tag")]
struct Tag {
    #[kbin(attr)]
    id: u32,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "info")]
struct Info {
    title_name: String,
    title_yomigana: String,
    artist_name: String,
    artist_yomigana: String,
    ascii: String,
    bpm_max: u32,
    bpm_min: u32,
    distribution_date: u32,
    volume: u16,
    bg_no: u16,
    genre: u32,
    is_fixed: u8,
    version: u8,
    demo_pri: i8,
    inf_ver: u8,
    license_text: Option<String>,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "difficulty")]
struct Difficulty {
    novice: Chart,
    advanced: Chart,
    exhaust: Chart,
    infinite: Option<Chart>,
    maximum: Option<Chart>,
    ultimate: Option<Chart>,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "chart")]
struct Chart {
    difnum: u8,
    illustrator: String,
    effected_by: String,
    price: i32,
    limited: u8,
    jacket_print: i32,
    jacket_mask: i32,
    max_exscore: i32,
    radar: Radar,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "radar")]
struct Radar {
    notes: u8,
    peak: u8,
    tsumami: u8,
    tricky: u8,
    #[kbin(rename = "hand-trip")]
    hand_trip: u8,
    #[kbin(rename = "one-hand")]
    one_hand: u8,
}

#[test]
fn decode_full_musicdb_sample() {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/musicdb.xml"),
    )
    .unwrap();
    let options = DecodeOptions {
        max_nodes: 500_000,
        ..DecodeOptions::default()
    };
    let db = decode_xml::<MusicDb>(&bytes, options).unwrap();
    assert_eq!(db.music.len(), 2_244);
    assert_eq!(db.music[0].id, 1);
    assert_eq!(db.music[0].info.title_name, "ALBIDA Powerless Mix");
    assert_eq!(db.music.last().unwrap().id, 2_362);

    let encoded = encode_xml(
        &db,
        EncodeOptions {
            encoding: 0x80,
            names: NameMode::Full,
        },
    )
    .unwrap();
    let decoded = decode_xml::<MusicDb>(&encoded, options).unwrap();
    assert_eq!(decoded.music.len(), db.music.len());
    if let Some((index, (left, right))) = decoded
        .music
        .iter()
        .zip(&db.music)
        .enumerate()
        .find(|(_, (left, right))| left != right)
    {
        panic!("round-trip mismatch at music index {index}:\nleft={left:#?}\nright={right:#?}");
    }
}
