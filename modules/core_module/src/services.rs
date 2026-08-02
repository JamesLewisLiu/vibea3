use vibea3::{Kbin, RpcContext, RpcResult, rpc};

use crate::{State, fault};

const BUILTIN_SERVICES: &[&str] = &[
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
];

const SERVICE_EXPIRE_SECONDS: i32 = 10_800;
const OPERATION_MODE: &str = "operation";
const PRODUCT_DOMAIN: u8 = 1;
const CORE_MODULE: &str = "core-services";

#[derive(Kbin)]
#[kbin(node = "services")]
struct Request {
    info: Option<Info>,
}

#[derive(Kbin)]
#[kbin(node = "info")]
struct Info {
    #[kbin(rename = "AVS2")]
    avs2: Option<String>,
}

#[derive(Kbin)]
#[kbin(node = "services")]
struct Response {
    #[kbin(attr)]
    expire: i32,
    #[kbin(attr)]
    fault: Option<String>,
    #[kbin(attr)]
    mode: Option<String>,
    #[kbin(attr)]
    product_domain: Option<u8>,
    #[kbin(repeated)]
    item: Vec<Item>,
}

#[derive(Kbin)]
#[kbin(node = "item")]
struct Item {
    #[kbin(attr)]
    name: String,
    #[kbin(attr)]
    url: String,
}

#[rpc("services.get")]
async fn get(ctx: RpcContext<State>, request: Request) -> RpcResult<Response> {
    let modern = request
        .info
        .and_then(|info| info.avs2)
        .is_some_and(|version| version_at_least(&version, 2, 16));
    let base = if modern {
        &ctx.state.config.service_url
    } else {
        &ctx.state.config.legacy_service_url
    };
    let mut services: Vec<_> = BUILTIN_SERVICES
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                Some(CORE_MODULE.to_owned()),
                base.to_owned(),
            )
        })
        .collect();
    services.extend([
        ("ntp".to_owned(), None, ctx.state.config.ntp_url.clone()),
        (
            "keepalive".to_owned(),
            None,
            ctx.state.config.keepalive_url.clone(),
        ),
    ]);
    for endpoint in ctx.service_endpoints() {
        if !services
            .iter()
            .any(|(existing, _, _)| existing == &endpoint.name)
        {
            services.push((endpoint.name, Some(endpoint.module), base.to_owned()));
        }
    }
    Ok(Response {
        expire: SERVICE_EXPIRE_SECONDS,
        fault: fault(&ctx.model),
        mode: Some(OPERATION_MODE.into()),
        product_domain: Some(PRODUCT_DOMAIN),
        item: services
            .into_iter()
            .map(|(name, module, url)| Item {
                name,
                url: match (module, modern) {
                    (None, _) => url,
                    (Some(module), true) => module_url(&url, &module),
                    (Some(_), false) => format!("{}/+", base.trim_end_matches('/')),
                },
            })
            .collect(),
    })
}

fn module_url(base: &str, module: &str) -> String {
    let base = base.trim_end_matches('/');
    let authority = base.find("://").map_or(0, |index| index + 3);
    match base[authority..].rfind('/') {
        Some(relative) => format!("{}/{module}", &base[..authority + relative]),
        None => format!("{base}/{module}"),
    }
}

fn version_at_least(value: &str, major: u32, minor: u32) -> bool {
    let mut parts = value.split(['.', ' ']).filter_map(|part| part.parse().ok());
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0)) >= (major, minor)
}

#[cfg(test)]
mod tests {
    use super::module_url;

    #[test]
    fn module_url_replaces_the_shared_ea3_endpoint() {
        assert_eq!(
            module_url("http://127.0.0.1:5000/ea3", "core-services"),
            "http://127.0.0.1:5000/core-services"
        );
        assert_eq!(
            module_url("https://example.test/api/ea3/", "popn-highcheers"),
            "https://example.test/api/popn-highcheers"
        );
        assert_eq!(
            module_url("https://example.test", "services"),
            "https://example.test/services"
        );
    }
}
