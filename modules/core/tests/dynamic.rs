use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vibea3::{
    DbFuture, Document, DocumentDatabase, DynamicServerBuilder, FindOneAndUpdateOptions,
    FindOneOptions, HOST_API_FINGERPRINT, HostServices, IndexDefinition, ServiceConfig, Update,
    UpdateOptions,
};

struct Host(AtomicU32);

impl HostServices for Host {
    fn config(&self) -> ServiceConfig {
        ServiceConfig {
            instance_id: self.0.load(Ordering::Acquire).to_string(),
            service_url: "http://127.0.0.1/ea3".into(),
            legacy_service_url: "http://127.0.0.1/ea3".into(),
            web_ui_url: "http://127.0.0.1".into(),
            ntp_url: "ntp://127.0.0.1/".into(),
            keepalive_url: "ping://127.0.0.1/".into(),
            public_ip: [127, 0, 0, 1],
            maintenance: false,
            infinite_eacoin: false,
        }
    }

    fn database(&self) -> Arc<dyn DocumentDatabase> {
        Arc::new(UnavailableDatabase)
    }
}

struct UnavailableDatabase;

fn unavailable<T: Send + 'static>() -> DbFuture<T> {
    Box::pin(async { Err("database unavailable in loader test".into()) })
}

impl DocumentDatabase for UnavailableDatabase {
    fn create_indexes(&self, _collection: String, _indexes: Vec<IndexDefinition>) -> DbFuture<()> {
        unavailable()
    }
    fn find_one(
        &self,
        _collection: String,
        _filter: Document,
        _options: FindOneOptions,
    ) -> DbFuture<Option<Document>> {
        unavailable()
    }
    fn update_one(
        &self,
        _collection: String,
        _filter: Document,
        _update: Update,
        _options: UpdateOptions,
    ) -> DbFuture<()> {
        unavailable()
    }
    fn find_one_and_update(
        &self,
        _collection: String,
        _filter: Document,
        _update: Update,
        _options: FindOneAndUpdateOptions,
    ) -> DbFuture<Option<Document>> {
        unavailable()
    }
    fn insert_one(&self, _collection: String, _document: Document) -> DbFuture<()> {
        unavailable()
    }
    fn insert_many(&self, _collection: String, _documents: Vec<Document>) -> DbFuture<()> {
        unavailable()
    }
}

#[tokio::test]
async fn discovers_multiple_classes_and_serves_them() {
    let module = built_module();

    let empty = tempfile::tempdir().unwrap();
    let empty_services: Arc<dyn HostServices> = Arc::new(Host(AtomicU32::new(0)));
    let empty_server = DynamicServerBuilder::new(empty.path(), empty_services)
        .host_fingerprint(HOST_API_FINGERPRINT)
        .admin_token("secret")
        .watch(false)
        .build()
        .await
        .unwrap();
    assert!(empty_server.manager().statuses().is_empty());

    let invalid = tempfile::tempdir().unwrap();
    std::fs::write(invalid.path().join(module.file_name().unwrap()), b"invalid").unwrap();
    let invalid_services: Arc<dyn HostServices> = Arc::new(Host(AtomicU32::new(0)));
    assert!(
        DynamicServerBuilder::new(invalid.path(), invalid_services)
            .host_fingerprint(HOST_API_FINGERPRINT)
            .admin_token("secret")
            .watch(false)
            .build()
            .await
            .is_err()
    );

    let duplicate = tempfile::tempdir().unwrap();
    let extension = module.extension().unwrap();
    std::fs::copy(
        &module,
        duplicate.path().join("first").with_extension(extension),
    )
    .unwrap();
    std::fs::copy(
        &module,
        duplicate.path().join("second").with_extension(extension),
    )
    .unwrap();
    let duplicate_services: Arc<dyn HostServices> = Arc::new(Host(AtomicU32::new(0)));
    assert!(
        DynamicServerBuilder::new(duplicate.path(), duplicate_services)
            .host_fingerprint(HOST_API_FINGERPRINT)
            .admin_token("secret")
            .watch(false)
            .build()
            .await
            .is_err()
    );

    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join(module.file_name().unwrap());
    std::fs::copy(&module, &source).unwrap();
    let host = Arc::new(Host(AtomicU32::new(42)));
    let services: Arc<dyn HostServices> = host.clone();
    let server = DynamicServerBuilder::new(temporary.path(), services)
        .host_fingerprint(HOST_API_FINGERPRINT)
        .admin_token("secret")
        .watch(false)
        .build()
        .await
        .unwrap();
    let manager = server.manager();
    let statuses = manager.statuses();
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].model.as_deref(), Some("LDJ"));
    assert_eq!(statuses[0].datecode_min.as_deref(), Some("20251200"));
    assert_eq!(statuses[0].datecode_max.as_deref(), Some("20261200"));
    assert_eq!(statuses[0].services, ["game"]);
    assert_eq!(manager.service_names("LDJ:J:A:A:20260101"), ["game"]);
    assert_eq!(
        manager.service_endpoints("LDJ:J:A:A:20260101"),
        [vibea3::ServiceEndpoint {
            name: "game".into(),
            module: "core-services".into(),
        }]
    );
    assert!(manager.service_names("LDJ:J:A:A:20240101").is_empty());
    assert!(manager.service_names("M39:J:G:A:20260101").is_empty());
    assert_eq!(statuses[0].classes, ["cardmng", "pcbtracker"]);
    assert_eq!(
        statuses[0].routes,
        [
            "cardmng.inquire",
            "cardmng.panic",
            "cardmng.probe",
            "cardmng.recurse",
            "cardmng.slow",
            "pcbtracker.alive"
        ]
    );

    let app = server.into_router();
    let response = app
        .clone()
        .oneshot(
            Request::post("/?model=LDJ:J:A:A:20260101&module=cardmng&method=inquire")
                .body(Body::from(
                    "<call model='LDJ:J:A:A:20260101'><cardmng method='inquire'/></call>",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert!(
        String::from_utf8(body.to_vec())
            .unwrap()
            .contains(">42</value>")
    );
    assert!(
        rpc_model(&app, "LDJ:J:A:A:20240101", "inquire")
            .await
            .contains("method_not_found:cardmng.inquire")
    );
    assert!(
        rpc_model(&app, "M39:J:G:A:20260101", "inquire")
            .await
            .contains("method_not_found:cardmng.inquire")
    );

    let chained = app
        .clone()
        .oneshot(
            Request::post("/?model=LDJ:J:A:A:20260101&module=cardmng&method=probe")
                .body(Body::from(
                    "<call model='LDJ:J:A:A:20260101'><cardmng method='probe'/></call>",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(chained.into_body(), 1024 * 1024).await.unwrap();
    assert!(
        String::from_utf8(body.to_vec())
            .unwrap()
            .contains(">1</alive>")
    );
    assert!(
        rpc(&app, "panic")
            .await
            .contains("internal_error:module handler panicked")
    );
    assert_eq!(manager.statuses()[0].panics, 1);
    assert!(
        rpc(&app, "recurse")
            .await
            .contains("internal_error:RPC call depth exceeded")
    );

    let original = std::fs::read(&source).unwrap();
    std::fs::write(&source, b"not a dynamic library").unwrap();
    assert!(manager.reload("core-services").await.is_err());
    assert!(manager.statuses()[0].last_error.is_some());
    assert!(rpc(&app, "inquire").await.contains(">42</value>"));

    let old_generation = manager.statuses()[0].generation;
    let mut replacement = original.clone();
    replacement.push(0);
    std::fs::write(&source, replacement).unwrap();
    host.0.store(84, Ordering::Release);
    let old_call = tokio::spawn({
        let app = app.clone();
        async move { rpc(&app, "slow").await }
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let reload = tokio::spawn({
        let manager = manager.clone();
        async move { manager.reload("core-services").await }
    });
    tokio::time::timeout(Duration::from_secs(15), async {
        while manager.statuses()[0].generation == old_generation {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(rpc(&app, "inquire").await.contains(">84</value>"));
    assert!(old_call.await.unwrap().contains(">42</value>"));
    reload.await.unwrap().unwrap();
    assert_eq!(manager.statuses()[0].reloads, 1);
    assert!(manager.statuses()[0].last_error.is_none());

    let denied = app
        .clone()
        .oneshot(
            Request::get("/_vibea3/admin/v1/modules")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    let allowed = app
        .clone()
        .oneshot(
            Request::get("/_vibea3/admin/v1/modules")
                .header("Authorization", "Bearer secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);
    assert_eq!(allowed.headers()["Cache-Control"], "no-store");

    std::fs::remove_file(&source).unwrap();
    manager.rescan().await.unwrap();
    assert!(manager.statuses()[0].source_missing);
    assert!(rpc(&app, "inquire").await.contains(">84</value>"));

    let unloaded = app
        .clone()
        .oneshot(
            Request::post("/_vibea3/admin/v1/modules/core-services/unload")
                .header("Authorization", "Bearer secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unloaded.status(), StatusCode::OK);
    assert!(manager.statuses().is_empty());
    assert!(
        rpc(&app, "inquire")
            .await
            .contains("method_not_found:cardmng.inquire")
    );

    let watched_directory = tempfile::tempdir().unwrap();
    let watched_source = watched_directory.path().join(module.file_name().unwrap());
    std::fs::write(&watched_source, &original).unwrap();
    let watched_host = Arc::new(Host(AtomicU32::new(7)));
    let watched_services: Arc<dyn HostServices> = watched_host.clone();
    let watched_server = DynamicServerBuilder::new(watched_directory.path(), watched_services)
        .host_fingerprint(HOST_API_FINGERPRINT)
        .admin_token("secret")
        .build()
        .await
        .unwrap();
    let watched_manager = watched_server.manager();
    let watched_generation = watched_manager.statuses()[0].generation;
    watched_host.0.store(9, Ordering::Release);
    let mut changed = original;
    changed.extend_from_slice(b"watch");
    std::fs::write(&watched_source, changed).unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while watched_manager.statuses()[0].generation == watched_generation {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        rpc(&watched_server.into_router(), "inquire")
            .await
            .contains(">9</value>")
    );
}

async fn rpc(app: &axum::Router, method: &str) -> String {
    rpc_model(app, "LDJ:J:A:A:20260101", method).await
}

async fn rpc_model(app: &axum::Router, model: &str, method: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/?model={model}&module=cardmng&method={method}"))
                .body(Body::from(format!(
                    "<call model='{model}'><cardmng method='{method}'/></call>"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

fn built_module() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = if cfg!(windows) {
        "vibea3_test_module.dll"
    } else {
        "libvibea3_test_module.so"
    };
    let path = root.join("target").join("debug").join(file);
    let status = std::process::Command::new(env!("CARGO"))
        .args(["build", "-p", "vibea3-test-module"])
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(status.success());
    path
}
