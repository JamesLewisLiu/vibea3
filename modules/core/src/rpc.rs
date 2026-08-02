use std::{future::Future, net::SocketAddr, sync::Arc};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{HeaderValue, Method, Request, Response, StatusCode, header::CONTENT_TYPE},
};
use tracing::{debug, warn};

use crate::{
    Error, Result,
    codec::{
        DecodeOptions, EncodeOptions, NameMode, PacketFormat, decode_kbin_meta, decode_xml_meta,
        encode_kbin, encode_rpc_fault, encode_xml,
    },
    registry::{CallContext, InternalCaller, ServiceEndpoint},
    schema::{Field, FieldKind, Kbin, Op, Schema},
    transport::{self, Compression, TransportConfig},
};

pub type RpcResult<T> = core::result::Result<T, RpcError>;

const PACKET_LOG_TARGET: &str = "vibea3::rpc::packet";
const DEBUG_XML_OPTIONS: EncodeOptions = EncodeOptions {
    encoding: 0xa0,
    names: NameMode::Full,
};

pub struct RpcResponse<T> {
    pub status: i32,
    pub payload: T,
}

impl<T> RpcResponse<T> {
    pub fn new(status: i32, payload: T) -> Self {
        Self { status, payload }
    }
}

impl<T: Kbin> Kbin for RpcResponse<T> {
    type Builder = T::Builder;
    const SCHEMA: &'static Schema = T::SCHEMA;

    fn rpc_status(&self) -> i32 {
        self.status
    }
    fn encode(&self, encoder: &mut crate::codec::Encoder<'_>) -> Result<()> {
        self.payload.encode(encoder)
    }
    fn decode_field(
        builder: &mut Self::Builder,
        id: u16,
        decoder: &mut crate::codec::Decoder<'_, '_>,
    ) -> Result<()> {
        T::decode_field(builder, id, decoder)
    }
    fn finish(builder: Self::Builder) -> Result<Self> {
        Ok(Self::new(0, T::finish(builder)?))
    }
}

#[derive(Debug)]
pub struct RpcError {
    pub status: i32,
    pub fault: String,
    pub expire: Option<i32>,
}

impl RpcError {
    pub fn new(status: i32, fault: impl Into<String>) -> Self {
        Self {
            status,
            fault: fault.into(),
            expire: None,
        }
    }
    pub fn protocol_status(status: i32) -> Self {
        Self {
            status,
            fault: "0".into(),
            expire: Some(0),
        }
    }
    pub fn method_not_found(route: &str) -> Self {
        Self::new(1, format!("method_not_found:{route}"))
    }
}

impl From<Error> for RpcError {
    fn from(error: Error) -> Self {
        Self::new(1, format!("invalid_request:{error}"))
    }
}

#[derive(Clone)]
pub struct RpcContext<S> {
    pub state: S,
    pub model: String,
    pub class: String,
    pub method: String,
    pub srcid: Option<String>,
    pub tag: Option<String>,
    pub peer_addr: Option<SocketAddr>,
    pub(crate) internal: Option<Arc<dyn InternalCaller>>,
    pub(crate) depth: u8,
}

impl<S> RpcContext<S> {
    pub fn new(
        state: S,
        model: impl Into<String>,
        class: impl Into<String>,
        method: impl Into<String>,
    ) -> Self {
        Self {
            state,
            model: model.into(),
            class: class.into(),
            method: method.into(),
            srcid: None,
            tag: None,
            peer_addr: None,
            internal: None,
            depth: 0,
        }
    }
}

impl<S: core::fmt::Debug> core::fmt::Debug for RpcContext<S> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("RpcContext")
            .field("state", &self.state)
            .field("model", &self.model)
            .field("class", &self.class)
            .field("method", &self.method)
            .field("srcid", &self.srcid)
            .field("tag", &self.tag)
            .field("peer_addr", &self.peer_addr)
            .field("depth", &self.depth)
            .finish_non_exhaustive()
    }
}

impl<S> RpcContext<S> {
    pub fn service_endpoints(&self) -> Vec<ServiceEndpoint> {
        self.internal
            .as_ref()
            .map(|caller| caller.service_endpoints(&self.model))
            .unwrap_or_default()
    }

    pub async fn call<Req: Kbin + 'static, Resp: Kbin + 'static>(
        &self,
        route: &str,
        request: Req,
    ) -> RpcResult<Resp> {
        const MAX_DEPTH: u8 = 16;
        if self.depth >= MAX_DEPTH {
            return Err(RpcError::new(1, "internal_error:RPC call depth exceeded"));
        }
        let (class, method) = route
            .split_once('.')
            .filter(|(class, method)| {
                !class.is_empty() && !method.is_empty() && !method.contains('.')
            })
            .ok_or_else(|| RpcError::new(1, "internal_error:invalid internal route"))?;
        let caller = self
            .internal
            .clone()
            .ok_or_else(|| RpcError::new(1, "internal_error:internal dispatch is unavailable"))?;
        let options = EncodeOptions {
            encoding: 0xa0,
            names: NameMode::Full,
        };
        let envelope = Call {
            model: Some(self.model.clone()),
            srcid: self.srcid.clone(),
            tag: self.tag.clone(),
            payload: MethodPayload {
                method: method.to_owned(),
                payload: request,
            },
        };
        let bytes = encode_kbin(&envelope, options).map_err(RpcError::from)?;
        let outgoing = caller
            .call(
                route.to_owned(),
                CallContext {
                    model: self.model.clone(),
                    class: class.to_owned(),
                    method: method.to_owned(),
                    srcid: self.srcid.clone(),
                    tag: self.tag.clone(),
                    peer_addr: self.peer_addr,
                    internal: Some(caller.clone()),
                    depth: self.depth + 1,
                },
                Incoming {
                    bytes,
                    format: PacketFormat::Kbin,
                    encode: options,
                    compression: Compression::None,
                    eamuse_info: None,
                },
            )
            .await?;
        let response = crate::codec::decode_kbin::<ResponseEnvelope<Success<Resp>>>(
            &outgoing.bytes,
            DecodeOptions::default(),
        )
        .map_err(RpcError::from)?;
        Ok(response.payload.0)
    }
}

#[derive(Debug)]
pub struct Incoming {
    pub bytes: Vec<u8>,
    pub format: PacketFormat,
    pub encode: EncodeOptions,
    pub compression: Compression,
    pub eamuse_info: Option<String>,
}

#[derive(Debug)]
pub struct Outgoing {
    pub bytes: Vec<u8>,
    pub format: PacketFormat,
    pub compression: Compression,
    pub eamuse_info: Option<String>,
}

pub trait RpcDispatch<S>: Send + Sync + 'static {
    fn dispatch(
        &self,
        route: &str,
        ctx: RpcContext<S>,
        incoming: Incoming,
    ) -> impl Future<Output = RpcResult<Outgoing>> + Send;
}

pub async fn invoke<S, Req, Resp, F, Fut>(
    mut ctx: RpcContext<S>,
    incoming: Incoming,
    handler: F,
) -> RpcResult<Outgoing>
where
    S: Clone + Send + Sync + 'static,
    Req: Kbin,
    Resp: Kbin,
    F: FnOnce(RpcContext<S>, Req) -> Fut + Send,
    Fut: Future<Output = RpcResult<Resp>> + Send,
{
    let (call, meta) = match incoming.format {
        PacketFormat::Kbin => {
            decode_kbin_meta::<Call<Req>>(&incoming.bytes, DecodeOptions::default())
        }
        PacketFormat::Xml => {
            decode_xml_meta::<Call<Req>>(&incoming.bytes, DecodeOptions::default())
        }
    }
    .map_err(RpcError::from)?;
    let logged_method = meta.method.as_deref().unwrap_or("<missing>");
    debug!(
        target: PACKET_LOG_TARGET,
        direction = "in",
        route = %format_args!("{}.{}", ctx.class, ctx.method),
        model = call.model.as_deref().unwrap_or(&ctx.model),
        srcid = ?call.srcid,
        tag = ?call.tag,
        peer_addr = ?ctx.peer_addr,
        depth = ctx.depth,
        format = ?incoming.format,
        compression = ?incoming.compression,
        decoded_bytes = incoming.bytes.len(),
        representation = "normalized_xml",
        packet = %debug_input_packet(&call, Req::SCHEMA.node, logged_method),
        "XRPC input packet"
    );
    let body_method = meta
        .method
        .ok_or_else(|| RpcError::new(1, "invalid_request:missing method attribute"))?;
    if body_method != ctx.method {
        return Err(RpcError::new(
            1,
            "invalid_request:route/body method mismatch",
        ));
    }
    if let Some(model) = call.model {
        if !ctx.model.is_empty() && ctx.model != model {
            return Err(RpcError::new(
                1,
                "invalid_request:route/body model mismatch",
            ));
        }
        ctx.model = model;
    }
    ctx.srcid = call.srcid;
    ctx.tag = call.tag;
    let dstid = ctx.srcid.clone();
    let response_log = PacketLogContext {
        route: format!("{}.{}", ctx.class, ctx.method),
        model: ctx.model.clone(),
        srcid: ctx.srcid.clone(),
        tag: ctx.tag.clone(),
        peer_addr: ctx.peer_addr,
        depth: ctx.depth,
        format: incoming.format,
        compression: incoming.compression,
    };
    let response = handler(ctx, call.payload).await?;
    let status = response.rpc_status();
    let envelope = ResponseEnvelope {
        dstid,
        payload: Success(response),
    };
    debug!(
        target: PACKET_LOG_TARGET,
        direction = "out",
        route = %response_log.route,
        model = %response_log.model,
        srcid = ?response_log.srcid,
        tag = ?response_log.tag,
        peer_addr = ?response_log.peer_addr,
        depth = response_log.depth,
        format = ?response_log.format,
        compression = ?response_log.compression,
        status,
        representation = "normalized_xml",
        packet = %debug_packet(&envelope),
        "XRPC output packet"
    );
    let bytes = match incoming.format {
        PacketFormat::Kbin => encode_kbin(&envelope, incoming.encode),
        PacketFormat::Xml => encode_xml(&envelope, incoming.encode),
    }
    .map_err(RpcError::from)?;
    Ok(Outgoing {
        bytes,
        format: incoming.format,
        compression: incoming.compression,
        eamuse_info: incoming.eamuse_info,
    })
}

struct PacketLogContext {
    route: String,
    model: String,
    srcid: Option<String>,
    tag: Option<String>,
    peer_addr: Option<SocketAddr>,
    depth: u8,
    format: PacketFormat,
    compression: Compression,
}

fn debug_packet<T: Kbin>(packet: &T) -> String {
    match encode_xml(packet, DEBUG_XML_OPTIONS) {
        Ok(bytes) => String::from_utf8(bytes)
            .unwrap_or_else(|error| format!("<packet-log-error>{error}</packet-log-error>")),
        Err(error) => format!("<packet-log-error>{error}</packet-log-error>"),
    }
}

fn debug_input_packet<T: Kbin>(packet: &T, node: &str, method: &str) -> String {
    let mut xml = debug_packet(packet);
    let marker = format!("<{node}");
    let Some(start) = xml.find(&marker) else {
        return xml;
    };
    let Some(end) = xml[start..].find('>').map(|offset| start + offset) else {
        return xml;
    };
    let method = quick_xml::escape::escape(method);
    xml.insert_str(end, &format!(" method=\"{method}\""));
    xml
}

fn debug_fault_packet(class: &str, error: &RpcError) -> String {
    match encode_rpc_fault(
        PacketFormat::Xml,
        class,
        error.status,
        &error.fault,
        error.expire,
        DEBUG_XML_OPTIONS,
    ) {
        Ok(bytes) => String::from_utf8(bytes)
            .unwrap_or_else(|error| format!("<packet-log-error>{error}</packet-log-error>")),
        Err(error) => format!("<packet-log-error>{error}</packet-log-error>"),
    }
}

struct Call<T> {
    model: Option<String>,
    srcid: Option<String>,
    tag: Option<String>,
    payload: T,
}

struct CallBuilder<T> {
    model: Option<String>,
    srcid: Option<String>,
    tag: Option<String>,
    payload: Option<T>,
}

impl<T> Default for CallBuilder<T> {
    fn default() -> Self {
        Self {
            model: None,
            srcid: None,
            tag: None,
            payload: None,
        }
    }
}

impl<T: Kbin> Kbin for Call<T> {
    type Builder = CallBuilder<T>;
    const SCHEMA: &'static Schema = &Schema {
        node: "call",
        fields: &[
            Field {
                id: 0,
                name: "model",
                kind: FieldKind::Attribute,
                optional: true,
            },
            Field {
                id: 1,
                name: "srcid",
                kind: FieldKind::Attribute,
                optional: true,
            },
            Field {
                id: 2,
                name: "tag",
                kind: FieldKind::Attribute,
                optional: true,
            },
            Field {
                id: 3,
                name: T::SCHEMA.node,
                kind: FieldKind::Node,
                optional: false,
            },
        ],
        ops: &[
            Op::Enter("call"),
            Op::Field(0),
            Op::Field(1),
            Op::Field(2),
            Op::Field(3),
            Op::Exit,
        ],
    };

    fn encode(&self, encoder: &mut crate::codec::Encoder<'_>) -> Result<()> {
        if let Some(v) = &self.model {
            encoder.field("model", FieldKind::Attribute, v)?;
        }
        if let Some(v) = &self.srcid {
            encoder.field("srcid", FieldKind::Attribute, v)?;
        }
        if let Some(v) = &self.tag {
            encoder.field("tag", FieldKind::Attribute, v)?;
        }
        encoder.field(T::SCHEMA.node, FieldKind::Node, &self.payload)
    }

    fn decode_field(
        builder: &mut Self::Builder,
        id: u16,
        decoder: &mut crate::codec::Decoder<'_, '_>,
    ) -> Result<()> {
        match id {
            0 => builder.model = Some(decoder.value()?),
            1 => builder.srcid = Some(decoder.value()?),
            2 => builder.tag = Some(decoder.value()?),
            3 => builder.payload = Some(decoder.value()?),
            _ => return Err(Error::Schema("bad call field id".into())),
        }
        Ok(())
    }

    fn finish(builder: Self::Builder) -> Result<Self> {
        Ok(Self {
            model: builder.model,
            srcid: builder.srcid,
            tag: builder.tag,
            payload: builder.payload.ok_or(Error::Missing("module"))?,
        })
    }
}

struct Success<T>(T);

struct MethodPayload<T> {
    method: String,
    payload: T,
}

impl<T: Kbin> Kbin for MethodPayload<T> {
    type Builder = T::Builder;
    const SCHEMA: &'static Schema = T::SCHEMA;

    fn encode(&self, encoder: &mut crate::codec::Encoder<'_>) -> Result<()> {
        encoder.field("method", FieldKind::Attribute, &self.method)?;
        self.payload.encode(encoder)
    }

    fn decode_field(
        builder: &mut Self::Builder,
        id: u16,
        decoder: &mut crate::codec::Decoder<'_, '_>,
    ) -> Result<()> {
        T::decode_field(builder, id, decoder)
    }

    fn finish(builder: Self::Builder) -> Result<Self> {
        Ok(Self {
            method: String::new(),
            payload: T::finish(builder)?,
        })
    }
}

impl<T: Kbin> Kbin for Success<T> {
    type Builder = T::Builder;
    const SCHEMA: &'static Schema = T::SCHEMA;

    fn encode(&self, encoder: &mut crate::codec::Encoder<'_>) -> Result<()> {
        encoder.field("status", FieldKind::Attribute, &self.0.rpc_status())?;
        self.0.encode(encoder)
    }
    fn decode_field(
        builder: &mut Self::Builder,
        id: u16,
        decoder: &mut crate::codec::Decoder<'_, '_>,
    ) -> Result<()> {
        T::decode_field(builder, id, decoder)
    }
    fn finish(builder: Self::Builder) -> Result<Self> {
        T::finish(builder).map(Self)
    }
}

struct ResponseEnvelope<T> {
    dstid: Option<String>,
    payload: T,
}
struct ResponseBuilder<T> {
    dstid: Option<String>,
    payload: Option<T>,
}
impl<T> Default for ResponseBuilder<T> {
    fn default() -> Self {
        Self {
            dstid: None,
            payload: None,
        }
    }
}

impl<T: Kbin> Kbin for ResponseEnvelope<T> {
    type Builder = ResponseBuilder<T>;
    const SCHEMA: &'static Schema = &Schema {
        node: "response",
        fields: &[
            Field {
                id: 0,
                name: "dstid",
                kind: FieldKind::Attribute,
                optional: true,
            },
            Field {
                id: 1,
                name: T::SCHEMA.node,
                kind: FieldKind::Node,
                optional: false,
            },
        ],
        ops: &[Op::Enter("response"), Op::Field(0), Op::Field(1), Op::Exit],
    };
    fn encode(&self, encoder: &mut crate::codec::Encoder<'_>) -> Result<()> {
        if let Some(dstid) = &self.dstid {
            encoder.field("dstid", FieldKind::Attribute, dstid)?;
        }
        encoder.field(T::SCHEMA.node, FieldKind::Node, &self.payload)
    }
    fn decode_field(
        builder: &mut Self::Builder,
        id: u16,
        decoder: &mut crate::codec::Decoder<'_, '_>,
    ) -> Result<()> {
        match id {
            0 => builder.dstid = Some(decoder.value()?),
            1 => builder.payload = Some(decoder.value()?),
            _ => return Err(Error::Schema("bad response field id".into())),
        }
        Ok(())
    }
    fn finish(builder: Self::Builder) -> Result<Self> {
        Ok(Self {
            dstid: builder.dstid,
            payload: builder.payload.ok_or(Error::Missing("module"))?,
        })
    }
}

pub struct RpcServer<S, A> {
    state: S,
    app: A,
    transport: TransportConfig,
}

impl<S, A> RpcServer<S, A> {
    pub fn new(state: S, app: A) -> Self {
        Self {
            state,
            app,
            transport: TransportConfig::default(),
        }
    }
    pub fn with_transport(mut self, transport: TransportConfig) -> Self {
        self.transport = transport;
        self
    }
}

impl<S, A> RpcServer<S, A>
where
    S: Clone + Send + Sync + 'static,
    A: RpcDispatch<S>,
{
    pub fn into_router(self) -> Router {
        let shared = Arc::new(self);
        Router::new().fallback(move |request| handle_http(shared.clone(), request))
    }
}

async fn handle_http<S, A>(server: Arc<RpcServer<S, A>>, request: Request<Body>) -> Response<Body>
where
    S: Clone + Send + Sync + 'static,
    A: RpcDispatch<S>,
{
    match handle_http_inner(server, request).await {
        Ok(response) => response,
        Err((status, message)) => {
            warn!(status = status.as_u16(), error = %message, "XRPC request rejected");
            Response::builder()
                .status(status)
                .body(Body::from(message))
                .unwrap()
        }
    }
}

async fn handle_http_inner<S, A>(
    server: Arc<RpcServer<S, A>>,
    request: Request<Body>,
) -> core::result::Result<Response<Body>, (StatusCode, String)>
where
    S: Clone + Send + Sync + 'static,
    A: RpcDispatch<S>,
{
    if request.method() != Method::POST {
        return Err((StatusCode::METHOD_NOT_ALLOWED, "POST required".into()));
    }
    let route = parse_route(request.uri()).map_err(bad_request)?;
    let peer_addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0);
    let eamuse_info = request
        .headers()
        .get("x-eamuse-info")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let compression = Compression::parse(
        request
            .headers()
            .get("x-compress")
            .and_then(|v| v.to_str().ok()),
    )
    .map_err(bad_request)?;
    let body = to_bytes(request.into_body(), server.transport.max_body)
        .await
        .map_err(|e| bad_request(Error::Transport(e.to_string())))?;
    let wire_request_bytes = body.len();
    let decoded = transport::decode(
        &body,
        eamuse_info.as_deref(),
        compression,
        &server.transport,
    )
    .map_err(map_transport_error)?;
    debug!(
        target: PACKET_LOG_TARGET,
        direction = "in",
        route = %format_args!("{}.{}", route.class, route.method),
        model = %route.model,
        peer_addr = ?peer_addr,
        format = ?decoded.format,
        compression = ?compression,
        encrypted = eamuse_info.is_some(),
        wire_bytes = wire_request_bytes,
        decoded_bytes = decoded.bytes.len(),
        "XRPC input transport"
    );
    let encode = match decoded.format {
        PacketFormat::Kbin => EncodeOptions {
            encoding: decoded.bytes.get(2).copied().unwrap_or(0x80),
            names: match decoded.bytes.get(1) {
                Some(0x45 | 0x46) => NameMode::Full,
                _ => NameMode::Packed,
            },
        },
        PacketFormat::Xml => EncodeOptions {
            encoding: 0xa0,
            names: NameMode::Full,
        },
    };
    let format = decoded.format;
    let incoming = Incoming {
        bytes: decoded.bytes,
        format,
        encode,
        compression,
        eamuse_info: eamuse_info.clone(),
    };
    let route_model = route.model.clone();
    let ctx = RpcContext {
        state: server.state.clone(),
        model: route.model,
        class: route.class.clone(),
        method: route.method.clone(),
        srcid: None,
        tag: None,
        peer_addr,
        internal: None,
        depth: 0,
    };
    let key = format!("{}.{}", route.class, route.method);
    let outgoing = match server.app.dispatch(&key, ctx, incoming).await {
        Ok(outgoing) => outgoing,
        Err(error) => {
            debug!(
                target: PACKET_LOG_TARGET,
                direction = "out",
                route = %key,
                model = %route_model,
                peer_addr = ?peer_addr,
                format = ?format,
                compression = ?compression,
                status = error.status,
                fault = %error.fault,
                expire = ?error.expire,
                representation = "normalized_xml",
                packet = %debug_fault_packet(&route.class, &error),
                "XRPC output fault packet"
            );
            Outgoing {
                bytes: encode_rpc_fault(
                    format,
                    &route.class,
                    error.status,
                    &error.fault,
                    error.expire,
                    encode,
                )
                .map_err(bad_request)?,
                format,
                compression,
                eamuse_info: eamuse_info.clone(),
            }
        }
    };
    let encoded = transport::encode_adaptive(
        &outgoing.bytes,
        outgoing.eamuse_info.as_deref(),
        outgoing.compression,
        &server.transport,
    )
    .map_err(map_transport_error)?;
    debug!(
        target: PACKET_LOG_TARGET,
        direction = "out",
        route = %key,
        model = %route_model,
        peer_addr = ?peer_addr,
        format = ?outgoing.format,
        requested_compression = ?outgoing.compression,
        compression = ?encoded.compression,
        encrypted = outgoing.eamuse_info.is_some(),
        decoded_bytes = outgoing.bytes.len(),
        wire_bytes = encoded.bytes.len(),
        "XRPC output transport"
    );
    let mut response = Response::builder().status(StatusCode::OK);
    response = response.header(
        CONTENT_TYPE,
        match outgoing.format {
            PacketFormat::Kbin => "application/octet-stream",
            PacketFormat::Xml => "text/xml",
        },
    );
    response = response.header("X-Compress", encoded.compression.header());
    if let Some(info) = outgoing.eamuse_info {
        response = response.header(
            "X-Eamuse-Info",
            HeaderValue::from_str(&info)
                .map_err(|e| bad_request(Error::Transport(e.to_string())))?,
        );
    }
    Ok(response.body(Body::from(encoded.bytes)).unwrap())
}

struct Route {
    model: String,
    class: String,
    method: String,
}

fn parse_route(uri: &axum::http::Uri) -> Result<Route> {
    let path = uri.path();
    let slash_parts: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
    if slash_parts.len() >= 3 {
        let [.., model, class, method] = slash_parts.as_slice() else {
            unreachable!();
        };
        return Ok(Route {
            model: (*model).into(),
            class: (*class).into(),
            method: (*method).into(),
        });
    }
    let query: std::collections::HashMap<_, _> =
        form_urlencoded::parse(uri.query().unwrap_or("").as_bytes())
            .into_owned()
            .collect();
    let model = query.get("model").cloned().unwrap_or_default();
    if let Some(function) = query.get("f").or_else(|| query.get("function")) {
        let (class, method) = function
            .split_once('.')
            .ok_or_else(|| Error::Transport("invalid function query".into()))?;
        return Ok(Route {
            model,
            class: class.into(),
            method: method.into(),
        });
    }
    if path != "/" {
        let function = path.trim_start_matches('/');
        let (class, method) = function
            .split_once('.')
            .ok_or_else(|| Error::Transport("invalid function path".into()))?;
        return Ok(Route {
            model,
            class: class.into(),
            method: method.into(),
        });
    }
    if !query.contains_key("module") && !query.contains_key("method") {
        return Ok(Route {
            model,
            class: "services".into(),
            method: "get".into(),
        });
    }
    Ok(Route {
        model,
        class: query
            .get("module")
            .cloned()
            .ok_or_else(|| Error::Transport("missing module".into()))?,
        method: query
            .get("method")
            .cloned()
            .ok_or_else(|| Error::Transport("missing method".into()))?,
    })
}

fn bad_request(error: Error) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, error.to_string())
}
fn map_transport_error(error: Error) -> (StatusCode, String) {
    if matches!(error, Error::Limit) {
        (StatusCode::PAYLOAD_TOO_LARGE, error.to_string())
    } else {
        bad_request(error)
    }
}
