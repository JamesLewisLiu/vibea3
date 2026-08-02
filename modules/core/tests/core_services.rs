use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    },
};

use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, Version},
};
use bson::doc;
use tower::ServiceExt;
use vibea3::transport::{Compression, TransportConfig};
use vibea3::{
    DbFuture, DecodeOptions, Document, DocumentDatabase, DynamicServerBuilder, EncodeOptions,
    FindOneAndUpdateOptions, FindOneOptions, FindOptions, GeoLocation, HOST_API_FINGERPRINT,
    HostServices, IndexDefinition, Kbin, ServiceConfig, Update, UpdateOptions, decode_kbin,
    encode_kbin, transport,
};

const M39_MODEL: &str = "M39:U:D:A:2026041500";
const EAMUSE_INFO: &str = "1-12345678-abcd";
const SERVICES_RESPONSE_DUMP_ENV: &str = "VIBEA3_SERVICES_RESPONSE_DUMP";
const POPN_DATA_DIR_ENV: &str = "POPN_HIGHCHEERS_DATA_DIR";
const SERVICE_EXPIRE_SECONDS: i32 = 10_800;
const IDENTITY_DIGIT_COUNT: usize = 16;
const INFINITE_EACOIN_BALANCE: i32 = 9_999_999;
const CARD_STATUS_OK: &str = "0";
const CARD_STATUS_ERROR: &str = "1";
const CARD_STATUS_BANNED: &str = "109";
const CARD_STATUS_NOT_REGISTERED: &str = "112";
const CARD_STATUS_BAD_PIN: &str = "116";

#[derive(Kbin)]
#[kbin(node = "call")]
struct ServicesCall {
    #[kbin(attr)]
    model: String,
    #[kbin(attr)]
    srcid: String,
    #[kbin(attr)]
    tag: String,
    services: ServicesRequest,
}

#[derive(Kbin)]
#[kbin(node = "services")]
struct ServicesRequest {
    #[kbin(attr)]
    method: String,
    info: ServicesInfo,
}

#[derive(Kbin)]
#[kbin(node = "info")]
struct ServicesInfo {
    #[kbin(rename = "AVS2")]
    avs2: String,
}

#[derive(Debug, Kbin)]
#[kbin(node = "response")]
struct ServicesEnvelope {
    #[kbin(attr)]
    dstid: String,
    services: ServicesResponse,
}

#[derive(Debug, Kbin)]
#[kbin(node = "services")]
struct ServicesResponse {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: String,
    #[kbin(attr)]
    mode: String,
    #[kbin(attr)]
    product_domain: u8,
    #[kbin(attr)]
    status: i32,
    #[kbin(repeated)]
    item: Vec<ServiceItem>,
}

#[derive(Debug, Kbin)]
#[kbin(node = "item")]
struct ServiceItem {
    #[kbin(attr)]
    name: String,
    #[kbin(attr)]
    url: String,
}

struct Host {
    database: Arc<FakeDatabase>,
    infinite_eacoin: bool,
}

impl HostServices for Host {
    fn config(&self) -> ServiceConfig {
        ServiceConfig {
            instance_id: "test".into(),
            service_url: "http://services.example.com/ea3".into(),
            legacy_service_url: "http://legacy-services.example.com/ea3".into(),
            web_ui_url: "https://eagate.example.com".into(),
            ntp_url: "ntp://ntp.nict.jp/".into(),
            keepalive_url: "ping://127.0.0.1/".into(),
            public_ip: [127, 0, 0, 1],
            maintenance: false,
            infinite_eacoin: self.infinite_eacoin,
        }
    }

    fn database(&self) -> Arc<dyn DocumentDatabase> {
        self.database.clone()
    }

    fn geolocate(&self, _address: IpAddr) -> DbFuture<Option<GeoLocation>> {
        Box::pin(async {
            Ok(Some(GeoLocation {
                country: Some("JP".into()),
                country_name: Some("Japan".into()),
                country_jname: Some("日本".into()),
                region: Some("13".into()),
                region_name: Some("Tokyo".into()),
                region_jname: Some("東京都".into()),
                city_name: Some("Tokyo".into()),
                latitude: Some(35.689509),
                longitude: Some(139.69164),
            }))
        })
    }
}

struct FakeDatabase {
    writes: AtomicUsize,
    machine_exists: AtomicBool,
    eacoin_enabled: AtomicBool,
    maintenance: AtomicBool,
    card_case: AtomicU8,
    auth_case: AtomicU8,
    balance_exists: AtomicBool,
    consume_case: AtomicU8,
    binding_exists: AtomicBool,
    session_ttl_index: AtomicBool,
}

impl Default for FakeDatabase {
    fn default() -> Self {
        Self {
            writes: AtomicUsize::new(0),
            machine_exists: AtomicBool::new(true),
            eacoin_enabled: AtomicBool::new(true),
            maintenance: AtomicBool::new(false),
            card_case: AtomicU8::new(0),
            auth_case: AtomicU8::new(0),
            balance_exists: AtomicBool::new(true),
            consume_case: AtomicU8::new(0),
            binding_exists: AtomicBool::new(true),
            session_ttl_index: AtomicBool::new(false),
        }
    }
}

impl DocumentDatabase for FakeDatabase {
    fn create_indexes(&self, collection: String, indexes: Vec<IndexDefinition>) -> DbFuture<()> {
        if collection == "card_sessions"
            && indexes.iter().any(|index| {
                index.name.as_deref() == Some("session_expiry")
                    && index.expire_after_seconds == Some(0)
            })
        {
            self.session_ttl_index.store(true, Ordering::Relaxed);
        }
        Box::pin(async { Ok(()) })
    }

    fn find_one(
        &self,
        collection: String,
        filter: Document,
        _options: FindOneOptions,
    ) -> DbFuture<Option<Document>> {
        let machine_exists = self.machine_exists.load(Ordering::Relaxed);
        let eacoin_enabled = self.eacoin_enabled.load(Ordering::Relaxed);
        let maintenance = self.maintenance.load(Ordering::Relaxed);
        let card_case = self.card_case.load(Ordering::Relaxed);
        let auth_case = self.auth_case.load(Ordering::Relaxed);
        let balance_exists = self.balance_exists.load(Ordering::Relaxed);
        let consume_case = self.consume_case.load(Ordering::Relaxed);
        let binding_exists = self.binding_exists.load(Ordering::Relaxed);
        Box::pin(async move {
            let document = match collection.as_str() {
                "machines" if machine_exists => Some(doc! {
                    "_id": "00010203040506070809",
                    "eacoin_enabled": eacoin_enabled,
                    "maintenance": maintenance,
                    "facility": {
                        "id": "1", "country": "JP", "region": "JP-13",
                        "name": "Vibea3 Arcade", "country_name": "Japan",
                        "country_jname": "日本", "region_name": "Tokyo",
                        "region_jname": "東京都", "port": 5700_i32,
                        "latitude": 35_689_509_i32, "longitude": 139_691_640_i32,
                        "calendar_year": 2026_i32, "holidays": [],
                    }
                }),
                "machines" => None,
                "card_sessions" if filter.contains_key("_id") && auth_case != 2 => Some(doc! {
                    "_id": "1100000000000000", "card_id": "E004010000000000",
                    "user_id": "2200000000000000",
                    "tag": "", "logged_in": false, "updated_at": bson::DateTime::now(),
                    "expires_at": bson::DateTime::from_millis(i64::MAX),
                }),
                "card_sessions" if filter.contains_key("card_id") => Some(doc! {
                    "_id": "1100000000000000", "card_id": "E004010000000000",
                    "user_id": "2200000000000000",
                    "tag": "", "logged_in": false, "updated_at": bson::DateTime::now(),
                    "expires_at": bson::DateTime::from_millis(i64::MAX),
                }),
                "card_sessions" => None,
                "game_bindings" => binding_exists.then(|| {
                    doc! {
                        "_id": "LDJ:2200000000000000",
                        "user_id": "2200000000000000",
                        "model": "LDJ",
                    }
                }),
                "cards" if filter.get_bool("banned") == Ok(true) => {
                    (card_case == 2).then(|| card_document(true, true, 1_000, "1234"))
                }
                "cards" if filter.get_bool("active") == Ok(true) => {
                    if consume_case == 1 {
                        Some(card_document(true, false, 1_000, "1234"))
                    } else if auth_case == 2 || !balance_exists || consume_case == 2 {
                        None
                    } else {
                        Some(card_document(true, false, 1_000, "1234"))
                    }
                }
                "cards" => match card_case {
                    1 => None,
                    2 => Some(card_document(true, true, 1_000, "1234")),
                    3 => Some(card_document(false, false, 1_000, "1234")),
                    _ => Some(card_document(true, false, 1_000, "1234")),
                },
                _ => None,
            };
            Ok(document)
        })
    }

    fn find_many(
        &self,
        _collection: String,
        _filter: Document,
        _options: FindOptions,
    ) -> DbFuture<Vec<Document>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn update_one(
        &self,
        _collection: String,
        _filter: Document,
        _update: Update,
        _options: UpdateOptions,
    ) -> DbFuture<()> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(()) })
    }

    fn find_one_and_update(
        &self,
        collection: String,
        filter: Document,
        update: Update,
        _options: FindOneAndUpdateOptions,
    ) -> DbFuture<Option<Document>> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        let consume_case = self.consume_case.load(Ordering::Relaxed);
        let card_case = self.card_case.load(Ordering::Relaxed);
        Box::pin(async move {
            if collection != "cards" {
                return Ok(None);
            }
            if filter.contains_key("balance") {
                return Ok((consume_case == 0).then(|| card_document(true, false, 1_000, "1234")));
            }
            let inserted_user_id = match update {
                Update::Document(update) => update
                    .get_document("$setOnInsert")
                    .ok()
                    .and_then(|set| set.get_str("user_id").ok())
                    .map(str::to_owned),
                Update::Pipeline(_) => None,
            };
            let user_id = if card_case == 1 {
                inserted_user_id.as_deref().unwrap()
            } else {
                "2200000000000000"
            };
            let mut card = card_document(true, false, 0, "1234");
            card.insert("user_id", user_id);
            if let Ok(card_id) = filter.get_str("_id") {
                card.insert("_id", card_id);
            }
            Ok(Some(card))
        })
    }

    fn insert_one(&self, _collection: String, _document: Document) -> DbFuture<()> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(()) })
    }

    fn insert_many(&self, _collection: String, _documents: Vec<Document>) -> DbFuture<()> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(()) })
    }

    fn delete_many(&self, _collection: String, _filter: Document) -> DbFuture<()> {
        Box::pin(async { Ok(()) })
    }
}

fn card_document(active: bool, banned: bool, balance: i32, pin: &str) -> Document {
    doc! {
        "_id": "E004010000000000", "pin": pin, "active": active, "banned": banned,
        "user_id": "2200000000000000",
        "eacoin_enabled": true, "balance": balance, "auto_charge_enabled": false,
        "auto_charge_threshold": 0, "auto_charge_amount": 0,
    }
}

#[tokio::test]
async fn production_core_module_serves_all_core_routes() {
    let module = built_module();
    let directory = tempfile::tempdir().unwrap();
    std::fs::copy(&module, directory.path().join(module.file_name().unwrap())).unwrap();
    let database = Arc::new(FakeDatabase::default());
    let host: Arc<dyn HostServices> = Arc::new(Host {
        database: database.clone(),
        infinite_eacoin: false,
    });
    let server = DynamicServerBuilder::new(directory.path(), host)
        .host_fingerprint(HOST_API_FINGERPRINT)
        .admin_token("secret")
        .watch(false)
        .build()
        .await
        .unwrap();
    assert!(database.session_ttl_index.load(Ordering::Relaxed));
    assert_eq!(server.manager().statuses()[0].routes.len(), 18);
    let app = server.into_router();

    let services = call(
        &app,
        "services",
        "get",
        "<info><AVS2>2.16.1 r6901</AVS2></info>",
    )
    .await;
    assert!(services.contains("name=\"cardmng\""));
    assert!(services.contains("name=\"dlstatus\""));
    assert!(
        services.contains("name=\"package\" url=\"http://services.example.com/core-services\"")
    );
    assert!(
        services.contains("name=\"pcbtracker\" url=\"http://services.example.com/core-services\"")
    );
    assert!(services.contains("name=\"ntp\" url=\"ntp://ntp.nict.jp/\""));
    assert!(services.contains("name=\"keepalive\" url=\"ping://127.0.0.1/\""));
    assert!(!services.contains("name=\"posevent\""));
    assert!(!services.contains("name=\"userdata\""));
    assert!(!services.contains("name=\"local\""));
    assert!(services.contains("<response dstid=\"00010203040506070809\">"));
    let ibb_services = call_model(
        &app,
        "IBB:J:A:A:2010010100",
        "services",
        "get",
        "<info><AVS2>2.17.0</AVS2></info>",
    )
    .await;
    assert!(ibb_services.contains("mode=\"operation\""));
    assert!(ibb_services.contains("product_domain=\"1\""));
    let legacy_services = call_model(
        &app,
        "FDD:J:A:A",
        "services",
        "get",
        "<info><AVS2>2.13.6</AVS2></info>",
    )
    .await;
    assert!(legacy_services.contains("dstid=\"00010203040506070809\""));
    assert!(!legacy_services.contains("fault="));
    assert!(legacy_services.contains("url=\"http://legacy-services.example.com/ea3/+\""));
    assert!(legacy_services.contains("name=\"ntp\" url=\"ntp://ntp.nict.jp/\""));
    assert!(legacy_services.contains("name=\"keepalive\" url=\"ping://127.0.0.1/\""));
    let legacy_tracker = call_model(&app, "FDD:J:A:A", "pcbtracker", "alive", "").await;
    assert!(legacy_tracker.contains("dstid=\"00010203040506070809\""));
    assert!(!legacy_tracker.contains("fault="));
    let tracker = call(&app, "pcbtracker", "alive", "").await;
    assert!(tracker.contains("ecenable=\"1\""));
    assert!(
        call(&app, "message", "get", "")
            .await
            .contains("expire=\"600\"")
    );
    let facility = call(&app, "facility", "get", "").await;
    assert!(facility.contains("203.0.113.5"));
    assert!(facility.contains(">35689509</latitude>"));
    assert!(facility.contains(">N35.41.22.2</latitude>"));
    assert!(facility.contains(">E139.41.29.9</longitude>"));
    assert!(
        call(&app, "package", "list", "")
            .await
            .contains("secondary=\"0\"")
    );
    let pcb_event = call(
        &app,
        "pcbevent",
        "put",
        "<seq __type='u32'>0</seq><item><name>boot</name><value __type='s32'>1</value><time __type='time'>1</time></item>",
    )
    .await;
    assert!(pcb_event.contains("status=\"0\""));
    assert_eq!(database.writes.load(Ordering::Relaxed), 0);

    let checkin = call(
        &app,
        "eacoin",
        "checkin",
        "<cardid>E004010000000000</cardid>",
    )
    .await;
    assert!(checkin.contains(">1000</balance>"));
    assert!(checkin.contains(">0</point>"));
    assert!(
        call(
            &app,
            "eacoin",
            "checkout",
            "<sessid>E004010000000000</sessid>"
        )
        .await
        .contains("sessid")
    );
    let consume = call(
        &app,
        "eacoin",
        "consume",
        "<sessid>E004010000000000</sessid><payment __type='s32'>500</payment>",
    )
    .await;
    assert!(consume.contains(">500</balance>"));

    for class in ["system", "system_2", "system_3"] {
        let response = call(
            &app,
            class,
            "convcardnumber",
            "<data><card_id>E004010000000000</card_id></data>",
        )
        .await;
        assert!(response.contains("0PFCX4FY5XHY6715"));
    }
    database.card_case.store(3, Ordering::Relaxed);
    let getrefid = call_attrs(
        &app,
        "cardmng",
        "getrefid",
        "cardid='E004010000000000' passwd='1234'",
        "",
    )
    .await;
    assert_eq!(xml_attribute(&getrefid, "status"), Some(CARD_STATUS_OK));
    let first_ref_id = xml_attribute(&getrefid, "refid").unwrap();
    let first_data_id = xml_attribute(&getrefid, "dataid").unwrap();
    assert_eq!(first_data_id, "2200000000000000");
    assert_ne!(first_ref_id, first_data_id);
    let second_card = call_attrs(
        &app,
        "cardmng",
        "getrefid",
        "cardid='E004010000000002' passwd='1234'",
        "",
    )
    .await;
    assert_eq!(xml_attribute(&second_card, "dataid"), Some(first_data_id));
    assert_ne!(xml_attribute(&second_card, "refid"), Some(first_ref_id));
    database.card_case.store(1, Ordering::Relaxed);
    let new_card = call_attrs(
        &app,
        "cardmng",
        "getrefid",
        "cardid='E004010000000003' passwd='1234'",
        "",
    )
    .await;
    assert!(new_card.contains("status=\"0\""));
    let new_ref_id = xml_attribute(&new_card, "refid").unwrap();
    let new_data_id = xml_attribute(&new_card, "dataid").unwrap();
    assert_eq!(new_ref_id.len(), IDENTITY_DIGIT_COUNT);
    assert_eq!(new_data_id.len(), IDENTITY_DIGIT_COUNT);
    assert_ne!(new_ref_id, new_data_id);
    assert_ne!(new_data_id, first_data_id);
    database.card_case.store(0, Ordering::Relaxed);
    let active_registration = call_attrs(
        &app,
        "cardmng",
        "getrefid",
        "cardid='E004010000000000' passwd='1234'",
        "",
    )
    .await;
    assert_eq!(
        xml_attribute(&active_registration, "status"),
        Some(CARD_STATUS_ERROR)
    );
    let inquire = call_attrs(
        &app,
        "cardmng",
        "inquire",
        "cardid='E004010000000000' update='0'",
        "",
    )
    .await;
    assert_eq!(xml_attribute(&inquire, "status"), Some(CARD_STATUS_OK));
    assert!(inquire.contains("binded=\"1\""));
    database.binding_exists.store(false, Ordering::Relaxed);
    let unbound = call_attrs(
        &app,
        "cardmng",
        "inquire",
        "cardid='E004010000000000' update='0'",
        "",
    )
    .await;
    assert!(unbound.contains("binded=\"0\""));
    database.binding_exists.store(true, Ordering::Relaxed);
    let auth = call_attrs(
        &app,
        "cardmng",
        "authpass",
        "refid='1100000000000000' pass='1234'",
        "",
    )
    .await;
    assert_eq!(xml_attribute(&auth, "status"), Some(CARD_STATUS_OK));
    let bind = call_attrs(
        &app,
        "cardmng",
        "bindmodel",
        &format!("refid='{first_ref_id}'"),
        "",
    )
    .await;
    assert_eq!(xml_attribute(&bind, "status"), Some(CARD_STATUS_OK));
    assert_eq!(xml_attribute(&bind, "dataid"), Some(first_data_id));
    let data_list = call(&app, "cardmng", "getdatalist", "").await;
    assert_eq!(xml_attribute(&data_list, "status"), Some(CARD_STATUS_OK));
    assert!(
        call(&app, "dlstatus", "progress", "")
            .await
            .contains("dlstatus")
    );

    assert!(
        call(&app, "pcbtracker", "alive", "")
            .await
            .contains("status=\"0\"")
    );
    assert!(
        call_attrs(&app, "pcbtracker", "alive", "agree='1'", "")
            .await
            .contains("status=\"0\"")
    );
    assert!(
        call_model(&app, "LA9:J:A:A:2010010100", "pcbtracker", "alive", "")
            .await
            .contains("status=\"0\"")
    );
    database.eacoin_enabled.store(false, Ordering::Relaxed);
    assert!(
        call(&app, "pcbtracker", "alive", "")
            .await
            .contains("ecenable=\"0\"")
    );
    database.eacoin_enabled.store(true, Ordering::Relaxed);
    assert!(
        call_model(&app, "LMA:J:A:A:2010010100", "pcbtracker", "alive", "")
            .await
            .contains("ecenable=\"3\"")
    );
    database.maintenance.store(true, Ordering::Relaxed);
    assert!(
        call(&app, "message", "get", "")
            .await
            .contains("sys.eacoin.mainte")
    );
    database.machine_exists.store(false, Ordering::Relaxed);
    assert!(
        call(&app, "facility", "get", "")
            .await
            .contains("status=\"0\"")
    );
    assert!(
        call(&app, "pcbtracker", "alive", "")
            .await
            .contains("ecenable=\"1\"")
    );
    database.machine_exists.store(true, Ordering::Relaxed);

    let dummy = call_attrs(
        &app,
        "cardmng",
        "inquire",
        "cardid='0000000000000000' update='0'",
        "",
    )
    .await;
    assert!(dummy.contains("newflag=\"1\""));
    assert!(dummy.contains("ecflag=\"1\""));

    database.card_case.store(1, Ordering::Relaxed);
    let missing = call_attrs(
        &app,
        "cardmng",
        "inquire",
        "cardid='E004010000000001' update='0'",
        "",
    )
    .await;
    assert_eq!(
        xml_attribute(&missing, "status"),
        Some(CARD_STATUS_NOT_REGISTERED)
    );
    assert!(xml_attribute(&missing, "refid").is_none());
    database.card_case.store(2, Ordering::Relaxed);
    let banned = call_attrs(
        &app,
        "cardmng",
        "inquire",
        "cardid='E004010000000001' update='0'",
        "",
    )
    .await;
    assert_eq!(xml_attribute(&banned, "status"), Some(CARD_STATUS_BANNED));
    database.card_case.store(3, Ordering::Relaxed);
    let inactive = call_attrs(
        &app,
        "cardmng",
        "inquire",
        "cardid='E004010000000001' update='0'",
        "",
    )
    .await;
    assert_eq!(
        xml_attribute(&inactive, "status"),
        Some(CARD_STATUS_NOT_REGISTERED)
    );

    database.auth_case.store(1, Ordering::Relaxed);
    let bad_pin = call_attrs(
        &app,
        "cardmng",
        "authpass",
        "refid='1100000000000000' pass='9999'",
        "",
    )
    .await;
    assert_eq!(xml_attribute(&bad_pin, "status"), Some(CARD_STATUS_BAD_PIN));
    database.auth_case.store(2, Ordering::Relaxed);
    assert!(
        call_attrs(
            &app,
            "cardmng",
            "authpass",
            "refid='1100000000000000' pass='9999'",
            "",
        )
        .await
        .contains("status=\"1\"")
    );
    assert!(
        call_attrs(
            &app,
            "cardmng",
            "getrefid",
            "cardid='E004010000000000' passwd='12'",
            "",
        )
        .await
        .contains("status=\"1\"")
    );

    database.balance_exists.store(false, Ordering::Relaxed);
    assert!(
        call(
            &app,
            "eacoin",
            "checkin",
            "<cardid>E004010000000000</cardid>",
        )
        .await
        .contains("status=\"1\"")
    );
    database.consume_case.store(1, Ordering::Relaxed);
    let rejected_consume = call(
        &app,
        "eacoin",
        "consume",
        "<sessid>E004010000000000</sessid><payment __type='s32'>1500</payment>",
    )
    .await;
    assert!(rejected_consume.contains(">1</acstatus>"));
    assert!(rejected_consume.contains(">-500</balance>"));
    database.consume_case.store(2, Ordering::Relaxed);
    assert!(
        call(
            &app,
            "eacoin",
            "consume",
            "<sessid>E004010000000000</sessid><payment __type='s32'>500</payment>",
        )
        .await
        .contains("status=\"1\"")
    );

    for class in ["system", "system_2", "system_3"] {
        let response = call(
            &app,
            class,
            "convcardnumber",
            "<data><card_id>invalid</card_id></data>",
        )
        .await;
        assert!(response.contains("status=\"1\""));
        assert!(response.contains(">1</result>"));
    }

    let unlimited_directory = tempfile::tempdir().unwrap();
    std::fs::copy(
        &module,
        unlimited_directory.path().join(module.file_name().unwrap()),
    )
    .unwrap();
    let unlimited_host: Arc<dyn HostServices> = Arc::new(Host {
        database: database.clone(),
        infinite_eacoin: true,
    });
    let unlimited = DynamicServerBuilder::new(unlimited_directory.path(), unlimited_host)
        .host_fingerprint(HOST_API_FINGERPRINT)
        .admin_token("secret")
        .watch(false)
        .build()
        .await
        .unwrap()
        .into_router();
    let writes_before = database.writes.load(Ordering::Relaxed);
    let unlimited_checkin = call(
        &unlimited,
        "eacoin",
        "checkin",
        "<cardid>E004010000000000</cardid>",
    )
    .await;
    assert!(unlimited_checkin.contains(&format!(">{INFINITE_EACOIN_BALANCE}</balance>")));
    let unlimited_consume = call(
        &unlimited,
        "eacoin",
        "consume",
        "<sessid>E004010000000000</sessid><payment __type='s32'>500</payment>",
    )
    .await;
    assert!(unlimited_consume.contains(">0</acstatus>"));
    assert!(unlimited_consume.contains(&format!(">{INFINITE_EACOIN_BALANCE}</balance>")));
    assert_eq!(database.writes.load(Ordering::Relaxed), writes_before);
}

#[tokio::test]
async fn m39_services_get_uses_encrypted_lz77_kbin_transport() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let _data_directory = ScopedEnv::set(
        POPN_DATA_DIR_ENV,
        workspace.join("data").join("popn_highcheers"),
    );
    let modules = built_modules(&[("core", "core"), ("popn_highcheers", "popn_highcheers")]);
    let directory = tempfile::tempdir().unwrap();
    for module in modules {
        std::fs::copy(&module, directory.path().join(module.file_name().unwrap())).unwrap();
    }
    let host: Arc<dyn HostServices> = Arc::new(Host {
        database: Arc::new(FakeDatabase::default()),
        infinite_eacoin: false,
    });
    let app = DynamicServerBuilder::new(directory.path(), host)
        .host_fingerprint(HOST_API_FINGERPRINT)
        .admin_token("secret")
        .watch(false)
        .build()
        .await
        .unwrap()
        .into_router();

    let request_packet = encode_kbin(
        &ServicesCall {
            model: M39_MODEL.into(),
            srcid: "00010203040506070809".into(),
            tag: "m39-kbin-probe".into(),
            services: ServicesRequest {
                method: "get".into(),
                info: ServicesInfo {
                    avs2: "2.17.4".into(),
                },
            },
        },
        EncodeOptions::default(),
    )
    .unwrap();
    let transport_config = TransportConfig::default();
    let request_body = transport::encode(
        &request_packet,
        Some(EAMUSE_INFO),
        Compression::Lz77,
        &transport_config,
    )
    .unwrap();
    let response = app
        .oneshot(
            Request::post(format!("/?model={M39_MODEL}&f=services.get"))
                .version(Version::HTTP_10)
                .header("X-Compress", "lz77")
                .header("X-Eamuse-Info", EAMUSE_INFO)
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["X-Eamuse-Info"], EAMUSE_INFO);
    let compression = Compression::parse(
        response
            .headers()
            .get("X-Compress")
            .and_then(|value| value.to_str().ok()),
    )
    .unwrap();
    let response_body = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let decoded = transport::decode(
        &response_body,
        Some(EAMUSE_INFO),
        compression,
        &transport_config,
    )
    .unwrap();
    if let Some(path) = std::env::var_os(SERVICES_RESPONSE_DUMP_ENV) {
        std::fs::write(path, &decoded.bytes).unwrap();
    }
    let response =
        decode_kbin::<ServicesEnvelope>(&decoded.bytes, DecodeOptions::default()).unwrap();

    assert_eq!(response.dstid, "00010203040506070809");
    assert_eq!(response.services.status, 0);
    assert_eq!(response.services.expire, SERVICE_EXPIRE_SECONDS);
    assert_eq!(response.services.fault, "0");
    assert_eq!(response.services.mode, "operation");
    assert_eq!(response.services.product_domain, 1);
    assert!(
        response
            .services
            .item
            .iter()
            .all(|item| !item.url.is_empty())
    );
    for item in response
        .services
        .item
        .iter()
        .filter(|item| !matches!(item.name.as_str(), "ntp" | "keepalive"))
    {
        let module = if matches!(item.name.as_str(), "lobby2" | "local2" | "local3") {
            "popn-highcheers"
        } else {
            "core-services"
        };
        assert_eq!(item.url, format!("http://services.example.com/{module}"));
    }
    let names: Vec<_> = response
        .services
        .item
        .iter()
        .map(|item| item.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "services",
            "pcbtracker",
            "pcbevent",
            "message",
            "facility",
            "apsmanager",
            "sidmgr",
            "cardmng",
            "package",
            "dlstatus",
            "eacoin",
            "ins",
            "ntp",
            "keepalive",
            "lobby2",
            "local2",
            "local3",
        ]
    );
    let ntp = response
        .services
        .item
        .iter()
        .find(|item| item.name == "ntp")
        .unwrap();
    assert_eq!(ntp.url, "ntp://ntp.nict.jp/");
    let keepalive = response
        .services
        .item
        .iter()
        .find(|item| item.name == "keepalive")
        .unwrap();
    assert_eq!(keepalive.url, "ping://127.0.0.1/");
}

async fn call(app: &axum::Router, class: &str, method: &str, body: &str) -> String {
    call_attrs(app, class, method, "", body).await
}

async fn call_model(
    app: &axum::Router,
    model: &str,
    class: &str,
    method: &str,
    body: &str,
) -> String {
    call_model_attrs(app, model, class, method, "", body).await
}

async fn call_attrs(
    app: &axum::Router,
    class: &str,
    method: &str,
    attributes: &str,
    body: &str,
) -> String {
    call_model_attrs(app, "LDJ:J:A:A:2017082800", class, method, attributes, body).await
}

async fn call_model_attrs(
    app: &axum::Router,
    model: &str,
    class: &str,
    method: &str,
    attributes: &str,
    body: &str,
) -> String {
    let xml = format!(
        "<call model='{model}' srcid='00010203040506070809' tag='test'><{class} method='{method}' {attributes}>{body}</{class}></call>"
    );
    let mut request = Request::post(format!("/?model={model}&module={class}&method={method}"))
        .body(Body::from(xml))
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)),
        12345,
    )));
    let response = app.clone().oneshot(request).await.unwrap();
    let body = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

fn built_module() -> PathBuf {
    built_modules(&[("core", "core")]).pop().unwrap()
}

fn xml_attribute<'a>(document: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}=\"");
    let value = document.split_once(&marker)?.1;
    value.split_once('"').map(|(value, _)| value)
}

fn built_modules(packages: &[(&str, &str)]) -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command = std::process::Command::new(env!("CARGO"));
    command.arg("build");
    for (package, _) in packages {
        command.args(["-p", package]);
    }
    let status = command.current_dir(&root).status().unwrap();
    assert!(status.success());
    packages
        .iter()
        .map(|(_, library)| {
            let file = if cfg!(windows) {
                format!("{library}.dll")
            } else {
                format!("lib{library}.so")
            };
            root.join("target").join("debug").join(file)
        })
        .collect()
}

struct ScopedEnv {
    name: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl ScopedEnv {
    fn set(name: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(name);
        // This integration-test process is the only code mutating this dedicated variable.
        unsafe { std::env::set_var(name, value) };
        Self { name, previous }
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        // The guard restores the test process environment before this test returns.
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.name, previous);
            } else {
                std::env::remove_var(self.name);
            }
        }
    }
}
