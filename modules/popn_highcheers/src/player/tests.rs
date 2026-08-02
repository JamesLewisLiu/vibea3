use bson::DateTime;
use vibea3::{
    DecodeOptions, EncodeOptions, Kbin, decode_kbin, decode_xml, encode_kbin, encode_xml,
};

use super::methods::WriteRequest;
use super::music::WriteMusicRequest;
use super::profile::ReadResponse;
use super::profile_write::AccountWrite;
use super::social::{FriendData, FriendResponse, RankingResponse};
use super::start::StartResponse;
use crate::database::{EventState, PlayerProfile, UsageSnapshot};
use crate::info::InfoData;
use crate::info::wire::CommonResponse;

#[test]
fn required_profile_maps_round_trip_in_both_formats() {
    let profile = PlayerProfile {
        user_id: "1000000000000000".into(),
        g_pm_id: "000000000001".into(),
        name: "TEST".into(),
        pref: 13,
        tutorial: 308,
        read_news: 2,
        total_play_cnt: 7,
        latest_music: vec![12],
        nice: vec![34],
        favorite_chara: vec![56],
        popn_class: -1,
        power_point: 100,
        power_point_list: vec![1, 2],
        sc_news_no: -1,
        read_policy: 0,
        language: -1,
        eaappli_relation: 0,
        ep: 3,
        estatus: 4,
        customize: (0..12).collect(),
        option: Default::default(),
        config: Default::default(),
        items: Vec::new(),
        characters: Vec::new(),
        extra: Vec::new(),
        netvs: Default::default(),
        event: EventState::default(),
        lumina: 0,
        created_at: DateTime::now(),
        updated_at: DateTime::now(),
    };
    let response = ReadResponse::profile(&profile);
    let options = EncodeOptions::default();

    let xml = encode_xml(&response, options).unwrap();
    let text = String::from_utf8(xml.clone()).unwrap();
    assert!(text.contains("<g_pm_id __type=\"str\">000000000001</g_pm_id>"));
    assert!(text.contains("<name __type=\"str\">TEST</name>"));
    assert!(text.contains("<latest_music __type=\"s16\" __count=\"30\""));
    assert!(text.contains("<power_point_list __type=\"s32\" __count=\"20\""));
    assert!(text.contains("<popn_class __type=\"s8\">-1</popn_class>"));
    assert!(text.contains("<eaappli>"));
    assert!(text.contains("<customize>"));
    assert!(text.contains("<netvs"));
    let decoded = decode_xml::<ReadResponse>(&xml, DecodeOptions::default()).unwrap();
    assert_eq!(decoded.result, 0);
    let account = decoded.account.unwrap();
    assert_eq!(account.latest_music.len(), 30);
    assert_eq!(account.g_pm_id, "000000000001");
    assert_eq!(account.name, "TEST");

    let kbin = encode_kbin(&response, options).unwrap();
    let decoded = decode_kbin::<ReadResponse>(&kbin, DecodeOptions::default()).unwrap();
    assert_eq!(decoded.info.unwrap().ep, 3);
    assert_eq!(decoded.customize.unwrap().highlight, 11);
}

#[test]
fn write_parses_event_p29_and_preserves_arrays() {
    let xml = br#"<?xml version='1.0' encoding='UTF-8'?>
        <player>
          <ref_id __type='str'>1234567890123456</ref_id>
          <data_id __type='str'>1234567890123456</data_id>
          <shop_name __type='str'>SHOP</shop_name>
          <pref __type='s8'>13</pref>
          <account>
            <tutorial __type='s16'>308</tutorial>
            <nice __type='s16' __count='3'>1 2 3</nice>
          </account>
          <info><ep __type='u16'>9</ep></info>
          <event_p29>
            <basket_id __type='s16'>2</basket_id>
            <basket>
              <id __type='s16'>3</id>
              <point __type='u32'>400</point>
              <is_cleared __type='bool'>1</is_cleared>
            </basket>
            <ensta><checked __type='bool'>1</checked></ensta>
          </event_p29>
        </player>"#;
    let request = decode_xml::<WriteRequest>(xml, DecodeOptions::default()).unwrap();
    assert_eq!(request.account.unwrap().nice.unwrap(), vec![1, 2, 3]);
    assert_eq!(request.info.unwrap().ep, Some(9));
    let event = request.event_p29.unwrap();
    assert_eq!(event.basket_id, Some(2));
    assert_eq!(event.basket[0].point, 400);
    assert!(event.ensta.unwrap().checked);
}

#[test]
fn player_write_stage_schema_round_trips_all_captured_fields() {
    let xml = br#"<?xml version='1.0' encoding='UTF-8'?>
        <player>
          <ref_id __type='str'>2691439429209034</ref_id>
          <data_id __type='str'>4164070139934008</data_id>
          <shop_name __type='str'>SHOP</shop_name>
          <pref __type='s8'>13</pref>
          <stage>
            <no __type='s16'>2290</no>
            <sheet __type='u8'>1</sheet>
            <clear_rank __type='u8'>2</clear_rank>
            <clear_type __type='u8'>3</clear_type>
            <score __type='s32'>98765</score>
            <cool __type='s16'>100</cool>
            <great __type='s16'>20</great>
            <good __type='s16'>3</good>
            <bad __type='s16'>4</bad>
            <combo __type='s16'>123</combo>
            <highlight __type='s16'>5</highlight>
            <gauge __type='s16'>6</gauge>
            <gauge_type __type='s8'>3</gauge_type>
            <is_win __type='s8'>1</is_win>
            <matching __type='s8'>2</matching>
            <ojama_vs __type='s8'>-1</ojama_vs>
          </stage>
        </player>"#;
    let request = decode_xml::<WriteRequest>(xml, DecodeOptions::default()).unwrap();
    assert_eq!(request.stage.len(), 1);
    let stage = &request.stage[0];
    assert_eq!(stage.no, 2290);
    assert_eq!(stage.sheet, 1);
    assert_eq!(stage.clear_rank, 2);
    assert_eq!(stage.clear_type, 3);
    assert_eq!(stage.score, 98_765);
    assert_eq!(stage.cool, 100);
    assert_eq!(stage.great, 20);
    assert_eq!(stage.good, 3);
    assert_eq!(stage.bad, 4);
    assert_eq!(stage.combo, 123);
    assert_eq!(stage.highlight, 5);
    assert_eq!(stage.gauge, 6);
    assert_eq!(stage.gauge_type, 3);
    assert_eq!(stage.is_win, 1);
    assert_eq!(stage.matching, 2);
    assert_eq!(stage.ojama_vs, -1);

    let kbin = encode_kbin(&request, EncodeOptions::default()).unwrap();
    let decoded = decode_kbin::<WriteRequest>(&kbin, DecodeOptions::default()).unwrap();
    assert_eq!(decoded.stage.len(), 1);
    assert_eq!(decoded.stage[0].score, 98_765);
    assert_eq!(decoded.stage[0].ojama_vs, -1);
}

#[test]
fn audited_optional_branches_remain_in_compile_time_schemas() {
    assert_eq!(
        field_names::<WriteRequest>(),
        [
            "ref_id",
            "data_id",
            "shop_name",
            "pref",
            "account",
            "info",
            "stage",
            "config",
            "option",
            "item",
            "chara_param",
            "customize",
            "netvs",
            "ex_info",
            "event_p29",
        ]
    );
    assert_eq!(
        field_names::<AccountWrite>(),
        [
            "play_id",
            "start_type",
            "tutorial",
            "read_news",
            "latest_music",
            "nice",
            "favorite_chara",
            "popn_class",
            "power_point",
            "power_point_list",
            "sc_news_no",
            "read_policy",
            "language",
        ]
    );
    assert!(field_names::<ReadResponse>().contains(&"eaappli"));
    assert!(field_names::<ReadResponse>().contains(&"chara_param_old"));
    assert!(field_names::<WriteMusicRequest>().contains(&"my_graph"));
    assert_eq!(field_names::<FriendResponse>(), ["friend"]);
    assert_eq!(field_names::<FriendData>()[0], "con_flg");
    assert_eq!(
        field_names::<RankingResponse>(),
        ["all_ranking", "pref_ranking", "location_ranking"]
    );
    assert_eq!(
        &field_names::<StartResponse>()[1..],
        field_names::<CommonResponse>().as_slice()
    );
}

#[test]
fn disconnected_friend_flag_is_nested_under_friend() {
    let xml = encode_xml(&FriendResponse::disconnected(), EncodeOptions::default()).unwrap();
    let text = String::from_utf8(xml).unwrap();
    let friend = text.find("<friend>").unwrap();
    let flag = text.find("<con_flg ").unwrap();
    let close = text.find("</friend>").unwrap();
    assert!(friend < flag && flag < close);
    assert_eq!(text.matches("<con_flg ").count(), 1);
}

#[test]
fn common_popularity_is_runtime_data_not_static_config() {
    let usage = UsageSnapshot {
        popular_characters: vec![42, 7],
        popular_music: vec![2373, 100],
        recommend_music: vec![2373, 100]
            .into_iter()
            .chain(std::iter::repeat_n(-1, 28))
            .collect(),
    };
    let response = CommonResponse::new(&InfoData::default(), &usage);
    let xml = encode_xml(&response, EncodeOptions::default()).unwrap();
    let text = String::from_utf8(xml).unwrap();
    assert!(text.contains("<rank __type=\"s16\">1</rank>"));
    assert!(text.contains("<chara_num __type=\"s16\">42</chara_num>"));
    assert!(text.contains("<music_num __type=\"s16\">2373</music_num>"));
    assert!(text.contains("<music_no __type=\"s32\" __count=\"30\">"));
}

fn field_names<T: Kbin>() -> Vec<&'static str> {
    T::SCHEMA.fields.iter().map(|field| field.name).collect()
}
