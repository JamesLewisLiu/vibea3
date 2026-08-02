use vibea3::{
    Binary, DecodeOptions, EncodeOptions, Kbin, NameMode, WireString, decode_kbin, decode_xml,
    encode_kbin, encode_xml,
};

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "child")]
struct Child {
    enabled: bool,
    score: i16,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "test")]
struct Packet {
    #[kbin(attr)]
    method: Option<String>,
    id: u32,
    #[kbin(rename = "display_name")]
    name: String,
    #[kbin(default)]
    version: u32,
    data: Binary,
    #[kbin(array)]
    samples: Vec<i16>,
    position: [f32; 3],
    #[kbin(repeated)]
    children: Vec<Child>,
    child: Child,
}

fn packet() -> Packet {
    Packet {
        method: Some("run".into()),
        id: 0x1234_5678,
        name: "テスト".into(),
        version: 3,
        data: Binary(vec![1, 2, 3, 4].into()),
        samples: vec![-1, 2, 300],
        position: [1.25, -2.5, 9.0],
        children: vec![
            Child {
                enabled: false,
                score: 7,
            },
            Child {
                enabled: true,
                score: 9,
            },
        ],
        child: Child {
            enabled: true,
            score: -32,
        },
    }
}

#[test]
fn kbin_round_trip() {
    let source = packet();
    let encoded = encode_kbin(&source, EncodeOptions::default()).unwrap();
    assert_eq!(&encoded[..4], &[0xa0, 0x42, 0x80, 0x7f]);
    assert_eq!(
        decode_kbin::<Packet>(&encoded, DecodeOptions::default()).unwrap(),
        source
    );
}

#[test]
fn xml_round_trip() {
    let source = packet();
    let encoded = encode_xml(
        &source,
        EncodeOptions {
            encoding: 0xa0,
            ..EncodeOptions::default()
        },
    )
    .unwrap();
    let text = String::from_utf8(encoded.clone()).unwrap();
    assert!(text.contains("<test method=\"run\">"));
    assert!(text.contains("__type=\"u32\""));
    assert_eq!(
        decode_xml::<Packet>(&encoded, DecodeOptions::default()).unwrap(),
        source
    );
}

#[test]
fn full_name_kbin_round_trip() {
    let source = packet();
    let options = EncodeOptions {
        encoding: 0xa0,
        names: NameMode::Full,
    };
    let encoded = encode_kbin(&source, options).unwrap();
    assert_eq!(encoded[1], 0x45);
    assert_eq!(
        decode_kbin::<Packet>(&encoded, DecodeOptions::default()).unwrap(),
        source
    );
}

#[test]
fn xml_ignores_unknown_nodes_and_defaults_missing_fields() {
    #[derive(Debug, PartialEq, Kbin)]
    #[kbin(node = "simple")]
    struct Simple {
        value: u8,
        #[kbin(default)]
        revision: u32,
    }
    let xml = b"<simple><unknown><nested __type='u8'>9</nested></unknown><value __type='u8'>7</value></simple>";
    assert_eq!(
        decode_xml::<Simple>(xml, DecodeOptions::default()).unwrap(),
        Simple {
            value: 7,
            revision: 0
        }
    );
}

#[test]
fn xml_matches_official_numeric_and_short_array_rules() {
    #[derive(Debug, PartialEq, Kbin)]
    #[kbin(node = "numbers")]
    struct Numbers {
        signed: i32,
        empty: u32,
        flag: bool,
        #[kbin(array)]
        values: Vec<i16>,
    }
    let xml = br#"<numbers>
        <signed __type="s32"> -0x10 </signed>
        <empty __type="u32"></empty>
        <flag __type="bool">01</flag>
        <values __type="s16" __count="3">7</values>
    </numbers>"#;
    assert_eq!(
        decode_xml::<Numbers>(xml, DecodeOptions::default()).unwrap(),
        Numbers {
            signed: -16,
            empty: 0,
            flag: true,
            values: vec![7, 0, 0],
        }
    );
}

#[test]
fn xml_matches_official_binary_size_padding() {
    #[derive(Debug, PartialEq, Kbin)]
    #[kbin(node = "binary")]
    struct BinaryPacket {
        value: Binary,
    }
    let xml = br#"<binary><value __type="bin" __size="3">AA BB</value></binary>"#;
    assert_eq!(
        decode_xml::<BinaryPacket>(xml, DecodeOptions::default()).unwrap(),
        BinaryPacket {
            value: Binary(vec![0xaa, 0xbb, 0].into()),
        }
    );
}

#[test]
fn xml_formats_official_non_finite_float_tokens() {
    #[derive(Kbin)]
    #[kbin(node = "floats")]
    struct Floats {
        positive: f32,
        negative: f64,
        nan: f32,
    }
    let encoded = encode_xml(
        &Floats {
            positive: f32::INFINITY,
            negative: f64::NEG_INFINITY,
            nan: f32::NAN,
        },
        EncodeOptions {
            encoding: 0xa0,
            ..EncodeOptions::default()
        },
    )
    .unwrap();
    let text = String::from_utf8(encoded).unwrap();
    assert!(text.contains("(+INF)"));
    assert!(text.contains("(-INF)"));
    assert!(text.contains("(QNaN)"));
}

#[test]
fn attributes_are_emitted_in_official_lexicographic_order() {
    #[derive(Kbin)]
    #[kbin(node = "attrs")]
    struct Attrs {
        #[kbin(attr)]
        z: String,
        #[kbin(attr)]
        a: String,
        #[kbin(attr)]
        middle: String,
    }
    let xml = encode_xml(
        &Attrs {
            z: "3".into(),
            a: "1".into(),
            middle: "2".into(),
        },
        EncodeOptions {
            encoding: 0xa0,
            ..EncodeOptions::default()
        },
    )
    .unwrap();
    let xml = String::from_utf8(xml).unwrap();
    assert!(xml.contains("<attrs a=\"1\" middle=\"2\" z=\"3\"></attrs>"));
}

#[test]
fn explicit_repeated_allows_xml_style_primitive_sequences() {
    #[derive(Debug, PartialEq, Kbin)]
    #[kbin(node = "values")]
    struct Values {
        #[kbin(repeated, rename = "value")]
        items: Vec<i32>,
    }
    let source = Values {
        items: vec![1, -2, 3],
    };
    let xml = encode_xml(
        &source,
        EncodeOptions {
            encoding: 0xa0,
            ..EncodeOptions::default()
        },
    )
    .unwrap();
    let xml_text = String::from_utf8(xml.clone()).unwrap();
    assert_eq!(xml_text.matches("<value __type=\"s32\">").count(), 3);
    assert!(!xml_text.contains("__count"));
    assert_eq!(
        decode_xml::<Values>(&xml, DecodeOptions::default()).unwrap(),
        source
    );

    let kbin = encode_kbin(&source, EncodeOptions::default()).unwrap();
    assert_eq!(
        decode_kbin::<Values>(&kbin, DecodeOptions::default()).unwrap(),
        source
    );
}

#[test]
fn malformed_header_is_rejected() {
    let mut encoded = encode_kbin(&packet(), EncodeOptions::default()).unwrap();
    encoded[3] ^= 1;
    assert!(decode_kbin::<Packet>(&encoded, DecodeOptions::default()).is_err());
}

#[test]
fn kbin_requires_file_terminator_and_full_data_consumption() {
    let encoded = encode_kbin(&packet(), EncodeOptions::default()).unwrap();
    let schema_len = u32::from_be_bytes(encoded[4..8].try_into().unwrap()) as usize;
    let file_end = encoded[8..8 + schema_len]
        .iter()
        .rposition(|byte| *byte == 0xff)
        .map(|position| position + 8)
        .unwrap();

    let mut missing_end = encoded.clone();
    missing_end[file_end] = 0;
    assert!(decode_kbin::<Packet>(&missing_end, DecodeOptions::default()).is_err());

    let mut extra_data = encoded;
    let data_len_at = 8 + schema_len;
    let data_len = u32::from_be_bytes(extra_data[data_len_at..data_len_at + 4].try_into().unwrap());
    extra_data.extend_from_slice(&[0; 4]);
    extra_data[data_len_at..data_len_at + 4].copy_from_slice(&(data_len + 4).to_be_bytes());
    assert!(decode_kbin::<Packet>(&extra_data, DecodeOptions::default()).is_err());
}

#[test]
fn kbin_skips_unknown_fixed_vectors_and_arrays() {
    #[derive(Kbin)]
    #[kbin(node = "evolving")]
    struct NewPacket {
        vector: [i16; 3],
        #[kbin(array)]
        samples: Vec<u32>,
        value: u8,
    }
    #[derive(Debug, PartialEq, Kbin)]
    #[kbin(node = "evolving")]
    struct OldPacket {
        value: u8,
    }

    let encoded = encode_kbin(
        &NewPacket {
            vector: [-1, 2, 3],
            samples: vec![4, 5, 6],
            value: 9,
        },
        EncodeOptions::default(),
    )
    .unwrap();
    assert_eq!(
        decode_kbin::<OldPacket>(&encoded, DecodeOptions::default()).unwrap(),
        OldPacket { value: 9 }
    );
}

#[test]
fn wire_string_preserves_binary_encoding() {
    #[derive(Debug, PartialEq, Kbin)]
    #[kbin(node = "raw")]
    struct RawPacket {
        text: WireString,
    }
    let source = RawPacket {
        text: WireString {
            bytes: b"raw bytes".as_slice().into(),
            encoding: 0x80,
        },
    };
    let encoded = encode_kbin(&source, EncodeOptions::default()).unwrap();
    assert_eq!(
        decode_kbin::<RawPacket>(&encoded, DecodeOptions::default()).unwrap(),
        source
    );
}

#[test]
fn kbin_variable_values_use_the_full_u32_length_prefix() {
    const FIRST_LENGTH_ABOVE_24_BITS: usize = 0x0100_0000;

    #[derive(Debug, PartialEq, Kbin)]
    #[kbin(node = "large")]
    struct LargePacket {
        data: Binary,
    }

    let source = LargePacket {
        data: Binary(vec![0x5a; FIRST_LENGTH_ABOVE_24_BITS].into()),
    };
    let encoded = encode_kbin(&source, EncodeOptions::default()).unwrap();
    let decoded: LargePacket = decode_kbin(&encoded, DecodeOptions::default()).unwrap();
    assert_eq!(decoded.data.0.len(), FIRST_LENGTH_ABOVE_24_BITS);
    assert_eq!(decoded.data.0[0], 0x5a);
}
