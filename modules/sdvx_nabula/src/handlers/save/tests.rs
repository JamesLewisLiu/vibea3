use super::*;
use vibea3::{DecodeOptions, EncodeOptions, NameMode, decode_kbin, decode_xml, encode_kbin};

#[test]
fn representative_save_packet_preserves_every_nested_group_in_kbin() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<game>
  <play_id __type="u32">7</play_id><refid __type="str">R</refid><locid __type="str">L</locid>
  <appeal_id __type="u16">12</appeal_id><skill_level __type="s16">4</skill_level>
  <variant_gate><earned_power __type="s32">9</earned_power><earned_element><notes __type="s32">1</notes></earned_element><over_radar __type="s32" __count="6">1 2 3 4 5 6</over_radar></variant_gate>
  <setting><hispeed __type="s32">650</hispeed><music_id __type="s32">100</music_id></setting>
  <item><info><type __type="u32">1</type><id __type="u32">2</id><param __type="u32">3</param></info></item>
  <param><info><type __type="s32">5</type><id __type="s32">9</id><param __type="s32" __count="3">6 7 8</param></info></param>
  <story><info><story_id __type="s32">1</story_id><progress_id __type="s32">2</progress_id><progress_param __type="s32">3</progress_param><clear_cnt __type="s32">4</clear_cnt><route_flg __type="u32">5</route_flg></info></story>
  <course><ssnid __type="s16">1</ssnid><crsid __type="s16">2</crsid><tr><st __type="s16">3</st></tr></course>
  <track><music_id __type="u32">100</music_id><music_type __type="u32">2</music_type><score __type="u32">9999999</score><exscore __type="u32">1234</exscore><clear_type __type="u32">4</clear_type><score_grade __type="u32">5</score_grade><critical __type="u32">1000</critical><near __type="u32">2</near><error __type="u32">1</error><judge __type="s32" __count="7">1 2 3 4 5 6 7</judge><matching><code __type="str">A</code><score __type="u32">1</score></matching></track>
</game>"#;
    let request: SaveRequest = decode_xml(xml, DecodeOptions::default()).unwrap();
    assert_eq!(request.track.len(), 1);
    assert_eq!(request.course.len(), 1);
    let packet = encode_kbin(
        &request,
        EncodeOptions {
            encoding: 0x80,
            names: NameMode::Packed,
        },
    )
    .unwrap();
    let decoded: SaveRequest = decode_kbin(&packet, DecodeOptions::default()).unwrap();
    assert_eq!(
        decoded.variant_gate.unwrap().over_radar.unwrap(),
        [1, 2, 3, 4, 5, 6]
    );
    assert_eq!(decoded.track[0].score, 9_999_999);
    assert_eq!(
        decoded.track[0].judge.as_deref(),
        Some([1, 2, 3, 4, 5, 6, 7].as_slice())
    );
    assert_eq!(decoded.track[0].matching.len(), 1);
    let param = &decoded.param.unwrap().info[0];
    assert_eq!(param.id, 9);
    assert_eq!(param.param, [6, 7, 8]);
}
