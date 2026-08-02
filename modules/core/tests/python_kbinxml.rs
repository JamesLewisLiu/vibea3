use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use vibea3::{DecodeOptions, EncodeOptions, Kbin, decode_kbin, decode_xml, encode_kbin};

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "call")]
struct Call {
    #[kbin(attr)]
    model: String,
    player: Player,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "player")]
struct Player {
    name: String,
    #[kbin(array)]
    scores: Vec<i32>,
}

fn python(script: &str, args: &[&Path]) -> Option<std::process::Output> {
    let mut command = Command::new("python");
    command
        .arg("-c")
        .arg(script)
        .args(args)
        .stdin(Stdio::null());
    command.output().ok()
}

#[test]
fn round_trip_with_python_kbinxml() {
    let Some(probe) = python("import kbinxml, lxml", &[]) else {
        eprintln!("skipping Python interoperability test: python is unavailable");
        return;
    };
    if !probe.status.success() {
        eprintln!("skipping Python interoperability test: kbinxml/lxml are unavailable");
        return;
    }

    let dir = std::env::temp_dir().join(format!("vibea3-kbinxml-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let xml_path = dir.join("input.xml");
    let python_kbin_path = dir.join("python.kbin");
    let rust_kbin_path = dir.join("rust.kbin");
    let python_xml_path = dir.join("python.xml");

    fs::write(
        &xml_path,
        br#"<?xml version="1.0" encoding="UTF-8"?>
<call model="KFC:J:A:A:2026080100">
  <player>
    <name __type="str">CAT &amp; MOUSE</name>
    <scores __type="s32" __count="4">-7 0 42 2147483647</scores>
  </player>
</call>"#,
    )
    .unwrap();

    let output = python(
        "from pathlib import Path; from kbinxml import KBinXML; Path(__import__('sys').argv[2]).write_bytes(KBinXML(Path(__import__('sys').argv[1]).read_bytes()).to_binary(encoding='cp932', compressed=True))",
        &[&xml_path, &python_kbin_path],
    )
    .unwrap();
    assert!(
        output.status.success(),
        "Python XML -> kbin failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = Call {
        model: "KFC:J:A:A:2026080100".into(),
        player: Player {
            name: "CAT & MOUSE".into(),
            scores: vec![-7, 0, 42, i32::MAX],
        },
    };
    let decoded = decode_kbin::<Call>(
        &fs::read(&python_kbin_path).unwrap(),
        DecodeOptions::default(),
    )
    .unwrap();
    assert_eq!(decoded, expected);

    fs::write(
        &rust_kbin_path,
        encode_kbin(&decoded, EncodeOptions::default()).unwrap(),
    )
    .unwrap();
    let output = python(
        "from pathlib import Path; from kbinxml import KBinXML; Path(__import__('sys').argv[2]).write_text(KBinXML(Path(__import__('sys').argv[1]).read_bytes()).to_text(), encoding='utf-8')",
        &[&rust_kbin_path, &python_xml_path],
    )
    .unwrap();
    assert!(
        output.status.success(),
        "Python rejected Rust kbin: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let decoded = decode_xml::<Call>(
        &fs::read(&python_xml_path).unwrap(),
        DecodeOptions::default(),
    )
    .unwrap();
    assert_eq!(decoded, expected);

    fs::remove_dir_all(dir).unwrap();
}
