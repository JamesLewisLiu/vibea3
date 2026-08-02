mod common;
mod database;
mod handlers;
mod protocol;

use std::sync::Arc;

use vibea3::{HOST_API_FINGERPRINT, HostServices, export_xrpc_module};

use crate::{common::CommonData, database::Database};

#[derive(Clone)]
struct State {
    database: Database,
    common: Arc<CommonData>,
}

async fn init(host: Arc<dyn HostServices>) -> Result<State, String> {
    let common = Arc::new(common::load()?);
    tracing::info!(
        release_code = %common.release_code,
        music = common.music.len(),
        appeal_cards = common.appeal_cards.len(),
        akaname_parts = common.akaname_parts.len(),
        "loaded SDVX Nebula local catalogs"
    );
    let database = Database::new(host.database());
    database.initialize().await?;
    Ok(State { database, common })
}

export_xrpc_module! {
    module: "sdvx-nebula",
    version: "0.1.0",
    model: "KFC",
    datecode_min: "20260714",
    services: ["local"],
    host: dyn HostServices,
    host_fingerprint: HOST_API_FINGERPRINT,
    state: State,
    init: init,
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    use super::*;

    const ROUTES: &[&str] = &[
        "game.sv7_buy",
        "game.sv7_common",
        "game.sv7_entry_e",
        "game.sv7_entry_s",
        "game.sv7_exception",
        "game.sv7_frozen",
        "game.sv7_hiscore",
        "game.sv7_load",
        "game.sv7_load_ap",
        "game.sv7_load_m",
        "game.sv7_load_r",
        "game.sv7_log",
        "game.sv7_lounge",
        "game.sv7_new",
        "game.sv7_play_e",
        "game.sv7_play_s",
        "game.sv7_sample",
        "game.sv7_save",
        "game.sv7_save_ap",
        "game.sv7_save_c",
        "game.sv7_save_campaign",
        "game.sv7_save_e",
        "game.sv7_save_fi",
        "game.sv7_save_m",
        "game.sv7_save_mega",
        "game.sv7_save_pb",
        "game.sv7_save_usta_link",
        "game.sv7_save_valgene",
        "game.sv7_serial",
        "game.sv7_shop",
    ];

    #[test]
    fn exports_every_sv7_route_recovered_from_the_client() {
        let mut actual: Vec<_> =
            vibea3::registry::registered_handlers(env!("CARGO_PKG_NAME"), type_name::<State>())
                .unwrap()
                .into_iter()
                .map(|handler| handler.route)
                .collect();
        actual.sort_unstable();
        assert_eq!(actual, ROUTES);
    }

    #[test]
    fn exports_the_open_ended_nebula_gate() {
        let export = unsafe { &*vibea3_xrpc_module_v2() };
        assert_eq!(export.model, Some("KFC"));
        assert_eq!(export.datecode_min, Some("20260714"));
        assert_eq!(export.datecode_max, None);
        assert_eq!(export.services, ["local"]);
    }
}
