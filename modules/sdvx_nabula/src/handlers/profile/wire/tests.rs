use bson::DateTime;
use vibea3::{DecodeOptions, EncodeOptions, decode_xml, encode_xml};

use super::*;

fn score() -> MusicScore {
    MusicScore {
        id: "U:1:2".into(),
        user_id: "U".into(),
        music_id: 1,
        music_type: 2,
        score: 9_900_000,
        exscore: 4_321,
        clear_type: 4,
        score_grade: 8,
        max_chain: 1_234,
        best_critical: 1_000,
        best_near: 2,
        best_error: 1,
        volforce: 555,
        just: 0,
        effective_rate: 100,
        btn_rate: 98,
        long_rate: 97,
        vol_rate: 96,
        mode: 0,
        start_option: 0,
        gauge_type: 0,
        notes_option: 0,
        online_num: 0,
        local_num: 0,
        challenge_type: 0,
        retry_cnt: 0,
        judge: Vec::new(),
        mix_id: 0,
        mix_like: false,
        matching: Vec::new(),
        play_count: 7,
        clear_count: 6,
        ultimate_chain_count: 2,
        perfect_ultimate_chain_count: 1,
        updated_at: DateTime::now(),
    }
}

#[test]
fn load_m_uses_the_clients_26_slot_layout() {
    let response = ScoreResponse::from(score());
    assert_eq!(response.param.len(), MUSIC_RECORD_PARAM_COUNT);
    assert_eq!(response.param[MUSIC_PARAM_MUSIC_ID], 1);
    assert_eq!(response.param[MUSIC_PARAM_MUSIC_TYPE], 2);
    assert_eq!(response.param[MUSIC_PARAM_PRIMARY_SCORE], 9_900_000);
    assert_eq!(response.param[MUSIC_PARAM_PRIMARY_EXSCORE], 4_321);
    assert_eq!(response.param[MUSIC_PARAM_PRIMARY_CLEAR_TYPE], 4);
    assert_eq!(response.param[MUSIC_PARAM_PRIMARY_GRADE], 8);
    assert_eq!(response.param[MUSIC_PARAM_PRIMARY_MAX_CHAIN], 1_234);
    assert_eq!(response.param[MUSIC_PARAM_PRIMARY_PLAY_COUNT], 7);
    assert_eq!(response.param[MUSIC_PARAM_PRIMARY_CLEAR_COUNT], 6);
    assert_eq!(response.param[MUSIC_PARAM_PRIMARY_UC_COUNT], 2);
    assert_eq!(response.param[MUSIC_PARAM_PRIMARY_PUC_COUNT], 1);
    assert_eq!(response.param[MUSIC_PARAM_VOLFORCE], 555);
    assert_eq!(response.param[MUSIC_PARAM_BUTTON_RATE], 98);
    assert_eq!(response.param[MUSIC_PARAM_LONG_RATE], 97);
    assert_eq!(response.param[MUSIC_PARAM_VOL_RATE], 96);
    assert!(response.param[12..23].iter().all(|value| *value == 0));
}

#[test]
fn profile_param_preserves_the_compound_key_and_wire_width() {
    let response = ParamResponse::from(PlayerParam {
        param_type: 2,
        id: 7,
        values: vec![1, 2, 3],
    });
    assert_eq!(response.param_type, 2);
    assert_eq!(response.id, 7);
    assert_eq!(response.param.len(), PARAMETER_VALUE_COUNT);
    assert_eq!(&response.param[..3], [1, 2, 3]);
}

#[test]
fn campaign_and_weekly_music_are_repeated_root_nodes() {
    let xml = br#"<game>
      <result __type="u8">0</result>
      <campaign><campaign_id __type="s32">1</campaign_id><jackpot_flg __type="bool">1</jackpot_flg></campaign>
      <campaign><campaign_id __type="s32">2</campaign_id><jackpot_flg __type="bool">0</jackpot_flg></campaign>
      <weekly_music><week_id __type="s32">3</week_id><music_id __type="s32">4</music_id><music_type __type="s32">5</music_type><exscore __type="s32">6</exscore><rank __type="s32">7</rank></weekly_music>
      <weekly_music><week_id __type="s32">8</week_id><music_id __type="s32">9</music_id><music_type __type="s32">10</music_type><exscore __type="s32">11</exscore><rank __type="s32">12</rank></weekly_music>
    </game>"#;
    let response: LoadResponse = decode_xml(xml, DecodeOptions::default()).unwrap();
    assert_eq!(response.campaign.len(), 2);
    assert_eq!(response.weekly_music.len(), 2);

    let encoded = encode_xml(&response, EncodeOptions::default()).unwrap();
    let encoded = String::from_utf8(encoded).unwrap();
    assert_eq!(encoded.matches("<campaign>").count(), 2);
    assert_eq!(encoded.matches("<weekly_music>").count(), 2);
    assert!(!encoded.contains("<campaign><info>"));
}
