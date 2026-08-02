mod admin;
mod loader;

const DATECODE_DIGIT_COUNT: usize = 8;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use arc_swap::ArcSwap;
use axum::Router;
use futures_util::FutureExt;
use libloading::Library;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use thiserror::Error;
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tracing::{debug, error, info, warn};

use crate::{
    registry::{
        CallContext, ErasedHandler, ErasedState, HandlerFuture, InternalCaller, ServiceEndpoint,
    },
    rpc::{Incoming, Outgoing, RpcContext, RpcDispatch, RpcError, RpcResult, RpcServer},
    transport::TransportConfig,
};

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("module I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("module loader failed: {0}")]
    Loader(String),
    #[error("module validation failed: {0}")]
    Validation(String),
    #[error("module initialization failed: {0}")]
    Init(String),
    #[error("module operation timed out: {0}")]
    Timeout(&'static str),
    #[error("unknown module {0}")]
    Unknown(String),
    #[error("module watcher failed: {0}")]
    Watcher(String),
}

#[derive(Clone, Debug)]
pub struct DynamicOptions {
    pub init_timeout: Duration,
    pub drain_timeout: Duration,
    pub watcher_debounce: Duration,
    pub stability_delay: Duration,
}

impl Default for DynamicOptions {
    fn default() -> Self {
        Self {
            init_timeout: Duration::from_secs(15),
            drain_timeout: Duration::from_secs(30),
            watcher_debounce: Duration::from_millis(500),
            stability_delay: Duration::from_millis(100),
        }
    }
}

pub struct DynamicServerBuilder<H: ?Sized> {
    directory: PathBuf,
    host: Arc<H>,
    host_fingerprint: Option<String>,
    admin_token: Option<String>,
    transport: TransportConfig,
    options: DynamicOptions,
    watch: bool,
}

impl<H: ?Sized + Send + Sync + 'static> DynamicServerBuilder<H> {
    pub fn new(directory: impl Into<PathBuf>, host: Arc<H>) -> Self {
        Self {
            directory: directory.into(),
            host,
            host_fingerprint: None,
            admin_token: None,
            transport: TransportConfig::default(),
            options: DynamicOptions::default(),
            watch: true,
        }
    }

    pub fn host_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.host_fingerprint = Some(fingerprint.into());
        self
    }

    pub fn admin_token(mut self, token: impl Into<String>) -> Self {
        self.admin_token = Some(token.into());
        self
    }

    pub fn transport(mut self, transport: TransportConfig) -> Self {
        self.transport = transport;
        self
    }

    pub fn options(mut self, options: DynamicOptions) -> Self {
        self.options = options;
        self
    }

    pub fn watch(mut self, enabled: bool) -> Self {
        self.watch = enabled;
        self
    }

    pub async fn build(self) -> Result<DynamicRpcServer<H>, ModuleError> {
        let host_fingerprint = self
            .host_fingerprint
            .ok_or_else(|| ModuleError::Validation("a host API fingerprint is required".into()))?;
        let admin_token = self
            .admin_token
            .ok_or_else(|| ModuleError::Validation("an admin bearer token is required".into()))?;
        if admin_token.is_empty() {
            return Err(ModuleError::Validation(
                "the admin bearer token cannot be empty".into(),
            ));
        }
        let manager =
            ModuleManager::new(self.directory, self.host, host_fingerprint, self.options)?;
        manager.load_initial().await?;
        if self.watch {
            manager.start_watcher()?;
        }
        Ok(DynamicRpcServer {
            manager,
            admin_token: Arc::from(admin_token),
            transport: self.transport,
        })
    }
}

pub struct DynamicRpcServer<H: ?Sized + Send + Sync + 'static> {
    manager: ModuleManager<H>,
    admin_token: Arc<str>,
    transport: TransportConfig,
}

impl<H: ?Sized + Send + Sync + 'static> DynamicRpcServer<H> {
    pub fn manager(&self) -> ModuleManager<H> {
        self.manager.clone()
    }

    pub fn into_router(self) -> Router {
        let rpc = RpcServer::new(
            (),
            DynamicDispatch {
                manager: self.manager.clone(),
            },
        )
        .with_transport(self.transport)
        .into_router();
        admin::router(self.manager, self.admin_token).merge(rpc)
    }
}

pub struct ModuleManager<H: ?Sized + Send + Sync + 'static> {
    inner: Arc<ManagerInner<H>>,
}

impl<H: ?Sized + Send + Sync + 'static> Clone for ModuleManager<H> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

struct ManagerInner<H: ?Sized + Send + Sync + 'static> {
    directory: PathBuf,
    cache: PathBuf,
    host: Arc<H>,
    host_fingerprint: String,
    options: DynamicOptions,
    routes: ArcSwap<BTreeMap<String, Vec<Arc<ModuleGeneration>>>>,
    modules: RwLock<BTreeMap<String, Arc<ModuleGeneration>>>,
    reload: AsyncMutex<()>,
    next_generation: AtomicU64,
    quarantine: Mutex<Vec<Arc<ModuleGeneration>>>,
    watcher: Mutex<Option<RecommendedWatcher>>,
}

struct ModuleGeneration {
    id: String,
    version: String,
    source: PathBuf,
    shadow: PathBuf,
    hash: String,
    generation: u64,
    compatibility: ModuleCompatibility,
    services: Vec<String>,
    classes: Vec<String>,
    handlers: BTreeMap<String, ErasedHandler>,
    state: ErasedState,
    shutdown: unsafe fn(ErasedState) -> crate::module::ShutdownFuture,
    loaded_at: u64,
    accepting: AtomicBool,
    missing: AtomicBool,
    active: AtomicUsize,
    calls: AtomicU64,
    reloads: AtomicU64,
    panics: AtomicU64,
    last_error: Mutex<Option<String>>,
    _library: Library,
}

#[derive(Clone, Debug, Default, Serialize)]
struct ModuleCompatibility {
    model: Option<String>,
    datecode_min: Option<String>,
    datecode_max: Option<String>,
}

impl ModuleCompatibility {
    fn matches(&self, caller: &str) -> bool {
        let mut parts = caller.split(':');
        let product = parts.next().unwrap_or_default();
        if self.model.as_deref().is_some_and(|model| model != product) {
            return false;
        }
        if self.datecode_min.is_none() && self.datecode_max.is_none() {
            return true;
        }
        let Some(raw) = parts.nth(3) else {
            return false;
        };
        let Some(datecode) = raw.get(..DATECODE_DIGIT_COUNT).filter(|value| {
            value.len() == DATECODE_DIGIT_COUNT && value.bytes().all(|byte| byte.is_ascii_digit())
        }) else {
            return false;
        };
        self.datecode_min
            .as_deref()
            .is_none_or(|minimum| datecode >= minimum)
            && self
                .datecode_max
                .as_deref()
                .is_none_or(|maximum| datecode <= maximum)
    }

    fn overlaps(&self, other: &Self) -> bool {
        if let (Some(left), Some(right)) = (&self.model, &other.model)
            && left != right
        {
            return false;
        }
        !self
            .datecode_max
            .as_deref()
            .zip(other.datecode_min.as_deref())
            .is_some_and(|(left, right)| left < right)
            && !other
                .datecode_max
                .as_deref()
                .zip(self.datecode_min.as_deref())
                .is_some_and(|(left, right)| left < right)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ModuleStatus {
    pub id: String,
    pub version: String,
    pub source: String,
    pub shadow: String,
    pub hash: String,
    pub generation: u64,
    pub model: Option<String>,
    pub datecode_min: Option<String>,
    pub datecode_max: Option<String>,
    pub services: Vec<String>,
    pub classes: Vec<String>,
    pub routes: Vec<String>,
    pub loaded_at_unix: u64,
    pub source_missing: bool,
    pub active_calls: usize,
    pub total_calls: u64,
    pub reloads: u64,
    pub panics: u64,
    pub last_error: Option<String>,
}

impl<H: ?Sized + Send + Sync + 'static> ModuleManager<H> {
    fn new(
        directory: PathBuf,
        host: Arc<H>,
        host_fingerprint: String,
        options: DynamicOptions,
    ) -> Result<Self, ModuleError> {
        std::fs::create_dir_all(&directory)?;
        let directory = std::fs::canonicalize(directory)?;
        let cache = directory.join(".vibea3-generations");
        std::fs::create_dir_all(&cache)?;
        Ok(Self {
            inner: Arc::new(ManagerInner {
                directory,
                cache,
                host,
                host_fingerprint,
                options,
                routes: ArcSwap::from_pointee(BTreeMap::new()),
                modules: RwLock::new(BTreeMap::new()),
                reload: AsyncMutex::new(()),
                next_generation: AtomicU64::new(1),
                quarantine: Mutex::new(Vec::new()),
                watcher: Mutex::new(None),
            }),
        })
    }

    async fn load_initial(&self) -> Result<(), ModuleError> {
        let _guard = self.inner.reload.lock().await;
        let mut staged: BTreeMap<String, Arc<ModuleGeneration>> = BTreeMap::new();
        for source in self.candidates()? {
            let generation = loader::stage(&self.inner, &source).await?;
            if staged.contains_key(&generation.id) {
                return Err(ModuleError::Validation(format!(
                    "duplicate module id {}",
                    generation.id
                )));
            }
            if let Some((class, owner)) = staged.values().find_map(|module| {
                conflicting_class(module, &generation).map(|class| (class, &module.id))
            }) {
                return Err(ModuleError::Validation(format!(
                    "class {class} overlaps caller compatibility with {owner}"
                )));
            }
            staged.insert(generation.id.clone(), generation);
        }
        let loaded: Vec<_> = staged.values().cloned().collect();
        self.publish(staged, None).await;
        for module in loaded {
            info!(
                module = %module.id,
                version = %module.version,
                generation = module.generation,
                routes = module.handlers.len(),
                source = %module.source.display(),
                "dynamic module loaded"
            );
        }
        Ok(())
    }

    pub async fn rescan(&self) -> Result<(), ModuleError> {
        let _guard = self.inner.reload.lock().await;
        let candidates = self.candidates()?;
        let present: BTreeSet<_> = candidates.iter().cloned().collect();
        for module in self.current_modules() {
            let missing = !present.contains(&module.source);
            let was_missing = module.missing.swap(missing, Ordering::AcqRel);
            if missing && !was_missing {
                warn!(module = %module.id, source = %module.source.display(), "dynamic module source disappeared; keeping last good generation");
            } else if !missing && was_missing {
                info!(module = %module.id, source = %module.source.display(), "dynamic module source reappeared");
            }
        }
        for source in candidates {
            let current = self
                .current_modules()
                .into_iter()
                .find(|module| module.source == source);
            let hash = loader::file_hash(&source)?;
            if current.as_ref().is_some_and(|module| module.hash == hash) {
                if let Some(current) = &current {
                    *current.last_error.lock().unwrap() = None;
                }
                continue;
            }
            let staged = match loader::stage(&self.inner, &source).await {
                Ok(staged) => staged,
                Err(error) => {
                    if let Some(current) = &current {
                        *current.last_error.lock().unwrap() = Some(error.to_string());
                    }
                    return Err(error);
                }
            };
            if let Some(current) = &current
                && current.id != staged.id
            {
                let error = ModuleError::Validation(format!(
                    "{} changed module id from {} to {}",
                    source.display(),
                    current.id,
                    staged.id
                ));
                *current.last_error.lock().unwrap() = Some(error.to_string());
                return Err(error);
            }
            if let Err(error) = self.install(staged).await {
                if let Some(current) = &current {
                    *current.last_error.lock().unwrap() = Some(error.to_string());
                }
                return Err(error);
            }
        }
        Ok(())
    }

    pub async fn reload(&self, id: &str) -> Result<(), ModuleError> {
        let _guard = self.inner.reload.lock().await;
        let current = self
            .module(id)
            .ok_or_else(|| ModuleError::Unknown(id.to_owned()))?;
        if loader::file_hash(&current.source)? == current.hash {
            *current.last_error.lock().unwrap() = None;
            return Ok(());
        }
        let staged = match loader::stage(&self.inner, &current.source).await {
            Ok(staged) => staged,
            Err(error) => {
                *current.last_error.lock().unwrap() = Some(error.to_string());
                return Err(error);
            }
        };
        if staged.id != id {
            let error = ModuleError::Validation(format!(
                "replacement changed module id from {id} to {}",
                staged.id
            ));
            *current.last_error.lock().unwrap() = Some(error.to_string());
            return Err(error);
        }
        if let Err(error) = self.install(staged).await {
            *current.last_error.lock().unwrap() = Some(error.to_string());
            return Err(error);
        }
        Ok(())
    }

    pub async fn unload(&self, id: &str) -> Result<(), ModuleError> {
        let _guard = self.inner.reload.lock().await;
        let old = self
            .module(id)
            .ok_or_else(|| ModuleError::Unknown(id.to_owned()))?;
        let mut modules = self.inner.modules.read().unwrap().clone();
        modules.remove(id);
        self.publish(modules, Some(old)).await;
        info!(module = %id, "dynamic module unloaded");
        Ok(())
    }

    pub fn statuses(&self) -> Vec<ModuleStatus> {
        self.current_modules()
            .into_iter()
            .map(|module| ModuleStatus {
                id: module.id.clone(),
                version: module.version.clone(),
                source: module.source.display().to_string(),
                shadow: module.shadow.display().to_string(),
                hash: module.hash.clone(),
                generation: module.generation,
                model: module.compatibility.model.clone(),
                datecode_min: module.compatibility.datecode_min.clone(),
                datecode_max: module.compatibility.datecode_max.clone(),
                services: module.services.clone(),
                classes: module.classes.clone(),
                routes: module.handlers.keys().cloned().collect(),
                loaded_at_unix: module.loaded_at,
                source_missing: module.missing.load(Ordering::Acquire),
                active_calls: module.active.load(Ordering::Acquire),
                total_calls: module.calls.load(Ordering::Relaxed),
                reloads: module.reloads.load(Ordering::Relaxed),
                panics: module.panics.load(Ordering::Relaxed),
                last_error: module.last_error.lock().unwrap().clone(),
            })
            .collect()
    }

    pub fn service_names(&self, model: &str) -> Vec<String> {
        self.service_endpoints(model)
            .into_iter()
            .map(|endpoint| endpoint.name)
            .collect()
    }

    pub fn service_endpoints(&self, model: &str) -> Vec<ServiceEndpoint> {
        self.current_modules()
            .into_iter()
            .filter(|module| {
                module.accepting.load(Ordering::Acquire) && module.compatibility.matches(model)
            })
            .flat_map(|module| {
                module
                    .services
                    .iter()
                    .cloned()
                    .map(|name| ServiceEndpoint {
                        name,
                        module: module.id.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .fold(BTreeMap::new(), |mut endpoints, endpoint| {
                endpoints
                    .entry(endpoint.name.clone())
                    .and_modify(|module: &mut String| {
                        if endpoint.module < *module {
                            module.clone_from(&endpoint.module);
                        }
                    })
                    .or_insert(endpoint.module);
                endpoints
            })
            .into_iter()
            .map(|(name, module)| ServiceEndpoint { name, module })
            .collect()
    }

    pub fn start_watcher(&self) -> Result<(), ModuleError> {
        let (send, mut receive) = mpsc::unbounded_channel();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok() {
                    let _ = send.send(());
                }
            })
            .map_err(|error| ModuleError::Watcher(error.to_string()))?;
        watcher
            .watch(&self.inner.directory, RecursiveMode::NonRecursive)
            .map_err(|error| ModuleError::Watcher(error.to_string()))?;
        *self.inner.watcher.lock().unwrap() = Some(watcher);
        let inner = Arc::downgrade(&self.inner);
        tokio::spawn(async move {
            while receive.recv().await.is_some() {
                let Some(inner) = inner.upgrade() else {
                    break;
                };
                tokio::time::sleep(inner.options.watcher_debounce).await;
                while receive.try_recv().is_ok() {}
                let manager = ModuleManager { inner };
                if let Err(error) = manager.rescan().await {
                    error!(%error, "dynamic module rescan failed");
                }
            }
        });
        Ok(())
    }

    async fn dispatch(
        &self,
        route: &str,
        context: CallContext,
        incoming: Incoming,
    ) -> RpcResult<Outgoing> {
        let class = route.split_once('.').map(|pair| pair.0).unwrap_or("");
        let generation = loop {
            let routes = self.inner.routes.load();
            let Some(generation) = routes.get(class).and_then(|generations| {
                generations
                    .iter()
                    .find(|generation| generation.compatibility.matches(&context.model))
                    .cloned()
            }) else {
                return Err(RpcError::method_not_found(route));
            };
            if !generation.accepting.load(Ordering::Acquire) {
                std::hint::spin_loop();
                continue;
            }
            generation.active.fetch_add(1, Ordering::AcqRel);
            if generation.accepting.load(Ordering::Acquire) {
                break generation;
            }
            generation.active.fetch_sub(1, Ordering::AcqRel);
        };
        struct Active(Arc<ModuleGeneration>);
        impl Drop for Active {
            fn drop(&mut self) {
                self.0.active.fetch_sub(1, Ordering::AcqRel);
            }
        }
        let active = Active(generation.clone());
        generation.calls.fetch_add(1, Ordering::Relaxed);
        debug!(
            module = %generation.id,
            generation = generation.generation,
            %route,
            model = %context.model,
            "dispatching RPC call"
        );
        let Some(handler) = generation.handlers.get(route).copied() else {
            return Err(RpcError::method_not_found(route));
        };
        let future = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handler(generation.state.clone(), context, incoming)
        }));
        let result = match future {
            Ok(future) => std::panic::AssertUnwindSafe(future).catch_unwind().await,
            Err(payload) => Err(payload),
        };
        let result = match result {
            Ok(Ok(outgoing)) => Ok(Outgoing {
                bytes: outgoing.bytes.as_slice().to_vec(),
                format: outgoing.format,
                compression: outgoing.compression,
                eamuse_info: outgoing.eamuse_info.as_deref().map(str::to_owned),
            }),
            Ok(Err(error)) if error.fault == "internal_error:module handler panicked" => {
                generation.panics.fetch_add(1, Ordering::Relaxed);
                error!(module = %generation.id, generation = generation.generation, %route, "module handler panicked");
                Err(RpcError::new(error.status, error.fault.as_str()))
            }
            Ok(Err(error)) => Err(RpcError {
                status: error.status,
                fault: error.fault.as_str().to_owned(),
                expire: error.expire,
            }),
            Err(_) => {
                generation.panics.fetch_add(1, Ordering::Relaxed);
                error!(module = %generation.id, generation = generation.generation, %route, "module handler panicked");
                Err(RpcError::new(1, "internal_error:module handler panicked"))
            }
        };
        drop(active);
        result
    }

    async fn install(&self, staged: Arc<ModuleGeneration>) -> Result<(), ModuleError> {
        let old = self.module(&staged.id);
        let reloaded = old.is_some();
        let installed = staged.clone();
        if let Some(old) = &old
            && old.source != staged.source
        {
            return Err(ModuleError::Validation(format!(
                "module id {} is already provided by {}",
                staged.id,
                old.source.display()
            )));
        }
        if let Some(old) = &old {
            staged.reloads.store(
                old.reloads.load(Ordering::Relaxed).saturating_add(1),
                Ordering::Relaxed,
            );
        }
        let mut modules = self.inner.modules.read().unwrap().clone();
        for module in modules.values() {
            if module.id == staged.id {
                continue;
            }
            if let Some(class) = conflicting_class(module, &staged) {
                return Err(ModuleError::Validation(format!(
                    "class {class} overlaps caller compatibility with {}",
                    module.id
                )));
            }
        }
        modules.insert(staged.id.clone(), staged);
        self.publish(modules, old).await;
        info!(
            module = %installed.id,
            version = %installed.version,
            generation = installed.generation,
            routes = installed.handlers.len(),
            source = %installed.source.display(),
            reloaded,
            "dynamic module installed"
        );
        Ok(())
    }

    async fn publish(
        &self,
        modules: BTreeMap<String, Arc<ModuleGeneration>>,
        old: Option<Arc<ModuleGeneration>>,
    ) {
        let mut routes = BTreeMap::new();
        for module in modules.values() {
            for class in &module.classes {
                routes
                    .entry(class.clone())
                    .or_insert_with(Vec::new)
                    .push(module.clone());
            }
        }
        self.inner.routes.store(Arc::new(routes));
        if let Some(old) = &old {
            old.accepting.store(false, Ordering::Release);
        }
        *self.inner.modules.write().unwrap() = modules;
        if let Some(old) = old {
            self.drain(old).await;
        }
    }

    async fn drain(&self, old: Arc<ModuleGeneration>) {
        let drained = tokio::time::timeout(self.inner.options.drain_timeout, async {
            while old.active.load(Ordering::Acquire) != 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_ok();
        if !drained {
            warn!(module = %old.id, generation = old.generation, "generation quarantined after drain timeout");
            self.inner.quarantine.lock().unwrap().push(old);
            return;
        }
        let shutdown = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            (old.shutdown)(old.state.clone())
        }));
        let Ok(shutdown) = shutdown else {
            warn!(module = %old.id, "module shutdown panicked");
            return;
        };
        match tokio::time::timeout(
            self.inner.options.drain_timeout,
            std::panic::AssertUnwindSafe(shutdown).catch_unwind(),
        )
        .await
        {
            Ok(Ok(Ok(()))) => {
                info!(module = %old.id, generation = old.generation, "module generation stopped")
            }
            Ok(Ok(Err(error))) => warn!(module = %old.id, %error, "module shutdown failed"),
            Ok(Err(_)) => warn!(module = %old.id, "module shutdown panicked"),
            Err(_) => {
                warn!(module = %old.id, "module shutdown timed out; generation quarantined");
                self.inner.quarantine.lock().unwrap().push(old);
            }
        }
    }

    fn candidates(&self) -> Result<Vec<PathBuf>, ModuleError> {
        let extension = if cfg!(windows) { "dll" } else { "so" };
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&self.inner.directory)? {
            let path = entry?.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|value| value.eq_ignore_ascii_case(extension))
            {
                files.push(std::fs::canonicalize(path)?);
            }
        }
        files.sort();
        Ok(files)
    }

    fn current_modules(&self) -> Vec<Arc<ModuleGeneration>> {
        self.inner
            .modules
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    fn module(&self, id: &str) -> Option<Arc<ModuleGeneration>> {
        self.inner.modules.read().unwrap().get(id).cloned()
    }
}

struct DynamicDispatch<H: ?Sized + Send + Sync + 'static> {
    manager: ModuleManager<H>,
}

impl<H: ?Sized + Send + Sync + 'static> RpcDispatch<()> for DynamicDispatch<H> {
    async fn dispatch(
        &self,
        _route: &str,
        ctx: RpcContext<()>,
        incoming: Incoming,
    ) -> RpcResult<Outgoing> {
        let route = format!("{}.{}", ctx.class, ctx.method);
        self.manager
            .dispatch(
                &route,
                CallContext {
                    model: ctx.model,
                    class: ctx.class,
                    method: ctx.method,
                    srcid: ctx.srcid,
                    tag: ctx.tag,
                    peer_addr: ctx.peer_addr,
                    internal: Some(Arc::new(self.manager.clone())),
                    depth: 0,
                },
                incoming,
            )
            .await
    }
}

impl<H: ?Sized + Send + Sync + 'static> InternalCaller for ModuleManager<H> {
    fn call(&self, route: String, context: CallContext, incoming: Incoming) -> HandlerFuture {
        let manager = self.clone();
        Box::pin(async move { manager.dispatch(&route, context, incoming).await })
    }

    fn service_endpoints(&self, model: &str) -> Vec<ServiceEndpoint> {
        ModuleManager::service_endpoints(self, model)
    }
}

fn conflicting_class(left: &ModuleGeneration, right: &ModuleGeneration) -> Option<String> {
    if !left.compatibility.overlaps(&right.compatibility) {
        return None;
    }
    left.classes
        .iter()
        .find(|class| right.classes.contains(class))
        .cloned()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::ModuleCompatibility;

    fn compatibility(
        model: Option<&str>,
        minimum: Option<&str>,
        maximum: Option<&str>,
    ) -> ModuleCompatibility {
        ModuleCompatibility {
            model: model.map(str::to_owned),
            datecode_min: minimum.map(str::to_owned),
            datecode_max: maximum.map(str::to_owned),
        }
    }

    #[test]
    fn model_and_datecode_filter_is_inclusive() {
        let filter = compatibility(Some("LDJ"), Some("20251200"), Some("20261200"));
        assert!(filter.matches("LDJ:J:A:A:20251200"));
        assert!(filter.matches("LDJ:J:A:A:2026010100"));
        assert!(filter.matches("LDJ:J:A:A:20261200"));
        assert!(!filter.matches("LDJ:J:A:A:20240101"));
        assert!(!filter.matches("LDJ:J:A:A:20270101"));
        assert!(!filter.matches("M39:J:G:A:20260101"));
        assert!(!filter.matches("LDJ:J:A:A:invalid"));
    }

    #[test]
    fn disjoint_datecode_variants_can_share_a_class() {
        let old = compatibility(Some("LDJ"), Some("20240101"), Some("20251200"));
        let new = compatibility(Some("LDJ"), Some("20251201"), Some("20261200"));
        let touching = compatibility(Some("LDJ"), Some("20251200"), Some("20261200"));
        let other_game = compatibility(Some("M39"), Some("20240101"), Some("20261200"));
        let wildcard = compatibility(None, None, None);
        assert!(!old.overlaps(&new));
        assert!(old.overlaps(&touching));
        assert!(!old.overlaps(&other_game));
        assert!(old.overlaps(&wildcard));
    }
}
