use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};
use tower::ServiceExt;
use vibea3::{
    DecodeOptions, EncodeOptions, Kbin, RpcContext, RpcResult, RpcServer, decode_kbin, encode_kbin,
    rpc, rpc_app,
    transport::{self, Compression, TransportConfig},
};

#[derive(Clone)]
struct State;

#[derive(Kbin)]
#[kbin(node = "services")]
struct RequestPacket {}

#[derive(Kbin)]
#[kbin(node = "services")]
struct ResponsePacket {
    count: u32,
}

#[derive(Kbin)]
#[kbin(node = "call")]
struct CallEnvelope {
    #[kbin(attr)]
    model: String,
    #[kbin(attr)]
    srcid: String,
    #[kbin(attr)]
    tag: String,
    services: CallServices,
}

#[derive(Kbin)]
#[kbin(node = "services")]
struct CallServices {
    #[kbin(attr)]
    method: String,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "response")]
struct ResponseEnvelope {
    services: ResponseServices,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "services")]
struct ResponseServices {
    #[kbin(attr)]
    status: i32,
    count: u32,
}

#[rpc("services.get")]
async fn services_get(
    _ctx: RpcContext<State>,
    _request: RequestPacket,
) -> RpcResult<ResponsePacket> {
    Ok(ResponsePacket { count: 7 })
}

#[tokio::test]
async fn encrypted_lz77_kbin_rpc_round_trip() {
    let app = RpcServer::new(State, rpc_app![services_get]).into_router();
    let call = CallEnvelope {
        model: "TEST".into(),
        srcid: "1".into(),
        tag: "a".into(),
        services: CallServices {
            method: "get".into(),
        },
    };
    let packet = encode_kbin(&call, EncodeOptions::default()).unwrap();
    let config = TransportConfig::default();
    let info = "1-12345678-abcd";
    let wrapped = transport::encode(&packet, Some(info), Compression::Lz77, &config).unwrap();
    let request = Request::post("/ea3?model=TEST&f=services.get")
        .header("X-Compress", "lz77")
        .header("X-Eamuse-Info", info)
        .body(Body::from(wrapped))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["X-Eamuse-Info"], info);
    let compression = Compression::parse(
        response
            .headers()
            .get("X-Compress")
            .and_then(|value| value.to_str().ok()),
    )
    .unwrap();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let decoded = transport::decode(&body, Some(info), compression, &config).unwrap();
    let response =
        decode_kbin::<ResponseEnvelope>(&decoded.bytes, DecodeOptions::default()).unwrap();
    assert_eq!(
        response,
        ResponseEnvelope {
            services: ResponseServices {
                status: 0,
                count: 7
            }
        }
    );
}

#[tokio::test]
async fn avs_service_url_prefix_route() {
    let app = RpcServer::new(State, rpc_app![services_get]).into_router();
    let request = Request::post("/api.v1/ea3/TEST/services/get")
        .header("X-Compress", "none")
        .body(Body::from(
            "<?xml version='1.0' encoding='UTF-8'?><call model='TEST'><services method='get'/></call>",
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert!(
        String::from_utf8(body.to_vec())
            .unwrap()
            .contains("<services status=\"0\">")
    );
}

#[tokio::test]
async fn incompressible_response_downgrades_header() {
    let app = RpcServer::new(State, rpc_app![services_get]).into_router();
    let request = Request::post("//TEST/services/get")
        .header("X-Compress", "lz77")
        .body(Body::from(
            transport::encode(
                b"<call model='TEST'><services method='get'/></call>",
                None,
                Compression::Lz77,
                &TransportConfig::default(),
            )
            .unwrap(),
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let advertised = response.headers()["X-Compress"]
        .to_str()
        .unwrap()
        .to_owned();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let decoded = transport::decode(
        &body,
        None,
        Compression::parse(Some(&advertised)).unwrap(),
        &TransportConfig::default(),
    )
    .unwrap();
    assert_eq!(decoded.format, vibea3::codec::PacketFormat::Xml);
}

#[tokio::test]
async fn legacy_root_defaults_to_services_get() {
    let app = RpcServer::new(State, rpc_app![services_get]).into_router();
    let request = Request::post("/")
        .body(Body::from(
            "<call model='FDD:J:A:A'><services method='get'/></call>",
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert!(
        String::from_utf8(body.to_vec())
            .unwrap()
            .contains("<services status=\"0\">")
    );
}

#[tokio::test]
async fn unknown_method_is_protocol_fault() {
    let app = RpcServer::new(State, rpc_app![services_get]).into_router();
    let request = Request::post("/?model=TEST&module=services&method=missing")
        .body(Body::from(
            "<call model='TEST'><services method='missing'/></call>",
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("status=\"1\""));
    assert!(text.contains("method_not_found:services.missing"));
}

#[tokio::test]
async fn xml_rpc_round_trip() {
    let app = RpcServer::new(State, rpc_app![services_get]).into_router();
    let request = Request::post("/?model=TEST&module=services&method=get")
        .header("X-Compress", "none")
        .body(Body::from("<?xml version='1.0' encoding='UTF-8'?><call model='TEST' srcid='1' tag='a'><services method='get'/></call>"))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["X-Compress"], "none");
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("<response dstid=\"1\">"));
    assert!(text.contains("<services status=\"0\">"));
    assert!(text.contains(">7</count>"));
}

#[tokio::test]
async fn short_function_query_selects_the_requested_route() {
    let app = RpcServer::new(State, rpc_app![services_get]).into_router();
    let request = Request::post("/?model=TEST&f=services.missing")
        .body(Body::from(
            "<call model='TEST'><services method='missing'/></call>",
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("method_not_found:services.missing"));
}

#[tokio::test(flavor = "current_thread")]
async fn debug_filter_logs_decoded_input_and_output_packets() {
    let logs = LogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .without_time()
        .compact()
        .with_writer(logs.clone())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let app = RpcServer::new(State, rpc_app![services_get]).into_router();
    let request = Request::post("/?model=TEST&f=services.get")
        .header("X-Compress", "none")
        .body(Body::from(
            "<?xml version='1.0' encoding='UTF-8'?><call model='TEST' srcid='1' tag='debug'><services method='get'/></call>",
        ))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let output = logs.text();
    assert!(
        output.contains("XRPC input transport"),
        "missing input transport log in {output}"
    );
    assert!(output.contains("XRPC input packet"));
    assert!(output.contains("XRPC output packet"));
    assert!(output.contains("XRPC output transport"));
    assert!(output.contains("route=services.get"));
    assert!(output.contains("normalized_xml"));
    assert!(output.contains("debug"));
    assert!(output.contains("method=\"get\""));
    assert!(output.contains("<count __type=\"u32\">7</count>"));
}

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl LogBuffer {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl<'writer> tracing_subscriber::fmt::MakeWriter<'writer> for LogBuffer {
    type Writer = LogWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        LogWriter(self.0.clone())
    }
}

struct LogWriter(Arc<Mutex<Vec<u8>>>);

impl Write for LogWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
