mod geo;
mod mongo;
mod workspace_modules;

use std::{
    error::Error,
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use geo::GeoIp;
use mongo::MongoStore;
use tower_http::{
    classify::ServerErrorsFailureClass,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, error, info};
use tracing_subscriber::{EnvFilter, fmt};
use vibea3::{
    DbFuture, DocumentDatabase, DynamicServerBuilder, GeoLocation, HOST_API_FINGERPRINT,
    HostServices, ServiceConfig,
};

const DEFAULT_LOG_FILTER: &str = "vibea3_server=info,vibea3=info,tower_http=info";
type AnyError = Box<dyn Error + Send + Sync>;

struct Host {
    config: ServiceConfig,
    database: Arc<dyn DocumentDatabase>,
    geoip: GeoIp,
}

impl HostServices for Host {
    fn config(&self) -> ServiceConfig {
        self.config.clone()
    }

    fn database(&self) -> Arc<dyn DocumentDatabase> {
        self.database.clone()
    }

    fn geolocate(&self, address: IpAddr) -> DbFuture<Option<GeoLocation>> {
        let result = self.geoip.lookup(address);
        Box::pin(async move { result })
    }
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let dotenv = load_dotenv()?;
    init_logging()?;
    if let Some(path) = dotenv {
        info!(path = %path.display(), "loaded environment file");
    }
    if let Err(error) = run().await {
        error!(%error, "server stopped with an error");
        return Err(error);
    }
    Ok(())
}

async fn run() -> Result<(), AnyError> {
    let directory = env_path("VIBEA3_MODULE_DIR").unwrap_or_else(|| "modules-bin".into());
    if env_bool_or("VIBEA3_BUILD_MODULES", cfg!(debug_assertions)) {
        workspace_modules::build_and_deploy(&directory, !cfg!(debug_assertions))?;
    }
    let token =
        std::env::var("VIBEA3_ADMIN_TOKEN").map_err(|_| "VIBEA3_ADMIN_TOKEN must be set")?;
    let address: SocketAddr = std::env::var("VIBEA3_LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:5000".into())
        .parse()?;
    let mongo_uri =
        std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://172.29.14.146:27017".into());
    let mongo_name = std::env::var("MONGODB_DATABASE").unwrap_or_else(|_| "vibea3".into());
    info!(database = %mongo_name, "connecting to MongoDB");
    let database = MongoStore::connect(&mongo_uri, &mongo_name).await?;
    info!(database = %mongo_name, "connected to MongoDB");
    let public_ip = std::env::var("VIBEA3_PUBLIC_IP")
        .unwrap_or_else(|_| "127.0.0.1".into())
        .parse::<Ipv4Addr>()?
        .octets();
    let default_url = format!("http://127.0.0.1:{}/ea3", address.port());
    let config = ServiceConfig {
        instance_id: std::env::var("VIBEA3_INSTANCE_ID")
            .unwrap_or_else(|_| std::process::id().to_string()),
        service_url: std::env::var("VIBEA3_SERVICE_URL").unwrap_or_else(|_| default_url.clone()),
        legacy_service_url: std::env::var("VIBEA3_LEGACY_SERVICE_URL")
            .unwrap_or_else(|_| default_url.clone()),
        web_ui_url: std::env::var("VIBEA3_WEB_URL")
            .unwrap_or_else(|_| format!("http://127.0.0.1:{}", address.port())),
        ntp_url: std::env::var("VIBEA3_NTP_URL").unwrap_or_else(|_| "ntp://ntp.nict.jp/".into()),
        keepalive_url: std::env::var("VIBEA3_KEEPALIVE_URL").unwrap_or_else(|_| {
            "ping://127.0.0.1/?ga=127.0.0.1&pa=127.0.0.1&ia=127.0.0.1&t1=2&t2=15&rt=4".into()
        }),
        public_ip,
        maintenance: env_bool("VIBEA3_MAINTENANCE"),
        infinite_eacoin: env_bool_or("VIBEA3_INFINITE_EACOIN", true),
    };
    let geoip_path = env_path("GEOIP_DATABASE");
    let geoip = GeoIp::open(geoip_path.as_deref())?;
    info!(
        listen = %address,
        module_directory = %directory.display(),
        database = %mongo_name,
        geoip_enabled = geoip_path.is_some(),
        maintenance = config.maintenance,
        infinite_eacoin = config.infinite_eacoin,
        "bootstrap configuration ready"
    );
    let host: Arc<dyn HostServices> = Arc::new(Host {
        config,
        database,
        geoip,
    });
    let app = DynamicServerBuilder::new(directory, host)
        .host_fingerprint(HOST_API_FINGERPRINT)
        .admin_token(token)
        .build()
        .await?
        .into_router()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(false),
                )
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(tower_http::LatencyUnit::Millis),
                )
                .on_failure(
                    |failure: ServerErrorsFailureClass,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        error!(%failure, latency_ms = latency.as_millis(), "HTTP request failed");
                    },
                ),
        );
    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(listen = %listener.local_addr()?, "vibea3 server listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn load_dotenv() -> Result<Option<PathBuf>, dotenvy::Error> {
    match dotenvy::dotenv() {
        Ok(path) => Ok(Some(path)),
        Err(dotenvy::Error::Io(error)) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn init_logging() -> Result<(), AnyError> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    match std::env::var("VIBEA3_LOG_FORMAT")
        .unwrap_or_else(|_| "compact".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "compact" => fmt().with_env_filter(filter).compact().try_init()?,
        "pretty" => fmt().with_env_filter(filter).pretty().try_init()?,
        "json" => fmt().with_env_filter(filter).json().try_init()?,
        value => return Err(format!("unsupported VIBEA3_LOG_FORMAT {value:?}").into()),
    }
    Ok(())
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

fn env_bool(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn env_bool_or(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}
