mod database;
mod info;
mod lobby;
mod pcb;
mod player;
mod protocol;

use std::sync::Arc;

use vibea3::{HOST_API_FINGERPRINT, HostServices, export_xrpc_module};

use crate::database::Database;

#[derive(Clone)]
struct State {
    database: Database,
    info: Arc<info::InfoData>,
}

async fn init(host: Arc<dyn HostServices>) -> Result<State, String> {
    let info = Arc::new(info::load()?);
    let database = Database::new(host.database());
    database.initialize().await?;
    Ok(State { database, info })
}

export_xrpc_module! {
    module: "popn-highcheers",
    version: "0.1.0",
    model: "M39",
    datecode_min: "20251218",
    services: ["local2", "local3", "lobby2"],
    host: dyn HostServices,
    host_fingerprint: HOST_API_FINGERPRINT,
    state: State,
    init: init,
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    use super::*;

    #[test]
    fn exports_complete_m39_route_set() {
        let mut routes: Vec<_> =
            vibea3::registry::registered_handlers(env!("CARGO_PKG_NAME"), type_name::<State>())
                .unwrap()
                .into_iter()
                .map(|handler| handler.route)
                .collect();
        routes.sort_unstable();
        assert_eq!(
            routes,
            vec![
                "info.common",
                "lobby24.delete",
                "lobby24.entry",
                "lobby24.getList",
                "lobby24.update",
                "loctest24.log",
                "pcb.boot",
                "pcb.dlstatus",
                "pcb.error",
                "pcb.write",
                "player.buy",
                "player.conversion",
                "player.delete",
                "player.end",
                "player.friend",
                "player.logout",
                "player.new",
                "player.read",
                "player.read_option",
                "player.read_score",
                "player.start",
                "player.tsumtsum",
                "player.update_ranking",
                "player.write",
                "player.write_course",
                "player.write_music",
            ]
        );
    }

    #[test]
    fn exports_open_ended_high_cheers_gate() {
        let export = unsafe { &*vibea3_xrpc_module_v2() };
        assert_eq!(export.model, Some("M39"));
        assert_eq!(export.datecode_min, Some("20251218"));
        assert_eq!(export.datecode_max, None);
        assert_eq!(export.services, ["local2", "local3", "lobby2"]);
    }
}
