mod card_manage;
mod pcb_tracker;

use std::sync::Arc;

use vibea3::{HOST_API_FINGERPRINT, HostServices, export_xrpc_module};

#[derive(Clone)]
struct State {
    value: u32,
}

async fn init(host: Arc<dyn HostServices>) -> Result<State, String> {
    Ok(State {
        value: host.config().instance_id.parse().unwrap_or_default(),
    })
}

async fn shutdown(_state: State) -> Result<(), String> {
    Ok(())
}

export_xrpc_module! {
    module: "core-services",
    version: "1.0.0",
    model: "LDJ",
    datecode_min: "20251200",
    datecode_max: "20261200",
    services: ["game"],
    host: dyn HostServices,
    host_fingerprint: HOST_API_FINGERPRINT,
    state: State,
    init: init,
    shutdown: shutdown,
}
