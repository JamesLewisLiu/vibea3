use vibea3::{
    DecodeOptions, EncodeOptions, Kbin, decode_kbin, decode_xml, encode_kbin, encode_xml,
};

use super::{boot::BootRequest, write::WriteRequest};

#[test]
fn boot_and_write_schemas_are_complete() {
    let boot: Vec<_> = BootRequest::SCHEMA
        .fields
        .iter()
        .map(|field| field.name)
        .collect();
    assert_eq!(boot.len(), 17);
    assert_eq!(boot[0], "loc_id");
    assert_eq!(boot[16], "etc");

    let write: Vec<_> = WriteRequest::SCHEMA
        .fields
        .iter()
        .map(|field| field.name)
        .collect();
    assert_eq!(
        write,
        [
            "pcb_status",
            "pcb_setting",
            "vc_setting",
            "pcb_card",
            "dlstatus",
        ]
    );
}

#[derive(Kbin)]
#[kbin(node = "call")]
struct BootCall {
    #[kbin(attr)]
    model: String,
    #[kbin(attr)]
    srcid: String,
    #[kbin(attr)]
    tag: String,
    pcb: BootRequest,
}

#[test]
fn captured_m39_boot_packet_round_trips_through_kbin() {
    let call = decode_xml::<BootCall>(
        include_bytes!("../../tests/fixtures/pcb-boot-m39-2026041500.xml"),
        DecodeOptions::default(),
    )
    .unwrap();

    let packet = encode_kbin(&call, EncodeOptions::default()).unwrap();
    let decoded = decode_kbin::<BootCall>(&packet, DecodeOptions::default()).unwrap();
    let normalized =
        String::from_utf8(encode_xml(&decoded, EncodeOptions::default()).unwrap()).unwrap();

    for field in [
        "loc_id",
        "loc_name",
        "country",
        "region",
        "customer",
        "company",
        "rom_number",
        "os_act",
        "etc",
    ] {
        assert!(
            normalized.contains(&format!("<{field} __type=\"str\"")),
            "{field} did not retain its string wire type: {normalized}"
        );
    }
    assert!(normalized.contains("<gip __type=\"ip4\">127.0.0.1</gip>"));
    assert!(normalized.contains("<e_drive __type=\"u64\">9223372036854775807</e_drive>"));
    assert!(
        normalized.contains("<etc __type=\"str\"/>")
            || normalized.contains("<etc __type=\"str\"></etc>")
    );
}
