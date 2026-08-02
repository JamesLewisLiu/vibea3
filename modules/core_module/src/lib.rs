mod card_manage;
mod database;
mod dl_status;
mod eacoin;
mod facility;
mod message;
mod package;
mod pcb_event;
mod pcb_tracker;
mod services;
mod system;

use std::sync::Arc;

use vibea3::export_xrpc_module;
use vibea3::{HOST_API_FINGERPRINT, HostServices, ServiceConfig};

use crate::database::Database;

const MODEL_CODE_LENGTH: usize = 9;
const CARD_ID_HEX_LENGTH: usize = 16;
const CARD_PIN_DIGIT_COUNT: usize = 4;
const REF_ID_DIGIT_COUNT: usize = 16;
const USER_ID_DIGIT_COUNT: usize = 16;
const EACOIN_SESSION_ID_LENGTH: usize = 16;

#[derive(Clone)]
struct State {
    config: ServiceConfig,
    database: Database,
    host: Arc<dyn HostServices>,
}

fn fault(model: &str) -> Option<String> {
    (model.len() != MODEL_CODE_LENGTH).then(|| "0".into())
}

async fn init(host: Arc<dyn HostServices>) -> Result<State, String> {
    let database = Database::new(host.database());
    database.initialize().await?;
    Ok(State {
        config: host.config(),
        database,
        host,
    })
}

export_xrpc_module! {
    module: "core-services",
    version: "1.0.0",
    host: dyn HostServices,
    host_fingerprint: HOST_API_FINGERPRINT,
    state: State,
    init: init,
}
