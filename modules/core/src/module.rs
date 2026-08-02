use std::{future::Future, pin::Pin};

use futures_util::FutureExt;

use crate::registry::{ErasedState, HandlerDescriptor};

pub const MODULE_MAGIC: [u8; 8] = *b"VIBXRPC\0";
pub const MODULE_ABI_VERSION: u32 = 2;
pub const MODULE_EXPORT_SYMBOL: &[u8] = b"vibea3_xrpc_module_v2\0";
pub const VIBEA3_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type InitFuture = Pin<Box<dyn Future<Output = Result<ErasedState, String>> + Send + 'static>>;
pub type ShutdownFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'static>>;

pub fn guard_init<F>(future: F) -> InitFuture
where
    F: Future<Output = Result<ErasedState, String>> + Send + 'static,
{
    Box::pin(async move {
        std::panic::AssertUnwindSafe(future)
            .catch_unwind()
            .await
            .map_err(|_| "module initialization panicked".to_owned())?
    })
}

pub fn guard_shutdown<F>(future: F) -> ShutdownFuture
where
    F: Future<Output = Result<(), String>> + Send + 'static,
{
    Box::pin(async move {
        std::panic::AssertUnwindSafe(future)
            .catch_unwind()
            .await
            .map_err(|_| "module shutdown panicked".to_owned())?
    })
}

#[repr(C)]
pub struct ModuleExport {
    pub magic: [u8; 8],
    pub abi_version: u32,
    pub struct_size: usize,
    pub vibea3_version: &'static str,
    pub build_fingerprint: fn() -> String,
    pub module_id: &'static str,
    pub module_version: &'static str,
    pub model: Option<&'static str>,
    pub datecode_min: Option<&'static str>,
    pub datecode_max: Option<&'static str>,
    pub services: &'static [&'static str],
    pub package: &'static str,
    pub host_type: fn() -> &'static str,
    pub host_fingerprint: &'static str,
    pub state_type: fn() -> &'static str,
    pub handlers: fn() -> Result<Vec<HandlerDescriptor>, String>,
    pub init: unsafe fn(*const ()) -> InitFuture,
    pub shutdown: unsafe fn(ErasedState) -> ShutdownFuture,
}

pub fn build_fingerprint() -> String {
    format!(
        "vibea3={};rustc={};target={};debug={};panic={}",
        env!("CARGO_PKG_VERSION"),
        env!("VIBEA3_RUSTC_VERSION"),
        env!("VIBEA3_TARGET"),
        cfg!(debug_assertions),
        if cfg!(panic = "unwind") {
            "unwind"
        } else {
            "abort"
        },
    )
}
