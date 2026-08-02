use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
    sync::{Arc, atomic::Ordering},
};

use futures_util::FutureExt;
use libloading::{Library, Symbol};
use sha2::{Digest, Sha256};

use super::{
    DATECODE_DIGIT_COUNT, ManagerInner, ModuleCompatibility, ModuleError, ModuleGeneration,
    unix_now,
};
use crate::module::{
    MODULE_ABI_VERSION, MODULE_EXPORT_SYMBOL, MODULE_MAGIC, ModuleExport, build_fingerprint,
};

pub(super) async fn stage<H: ?Sized + Send + Sync + 'static>(
    manager: &ManagerInner<H>,
    source: &Path,
) -> Result<Arc<ModuleGeneration>, ModuleError> {
    stable_file(source, manager.options.stability_delay).await?;
    let hash = file_hash(source)?;
    let generation = manager.next_generation.fetch_add(1, Ordering::Relaxed);
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ModuleError::Validation("module filename is not UTF-8".into()))?;
    let shadow = manager
        .cache
        .join(format!("{generation}-{hash}-{file_name}"));
    std::fs::copy(source, &shadow)?;

    let library =
        unsafe { Library::new(&shadow) }.map_err(|error| ModuleError::Loader(error.to_string()))?;
    let export = unsafe {
        let symbol: Symbol<'_, unsafe extern "C" fn() -> *const ModuleExport> = library
            .get(MODULE_EXPORT_SYMBOL)
            .map_err(|error| ModuleError::Loader(error.to_string()))?;
        let export = symbol();
        if export.is_null() {
            return Err(ModuleError::Validation(
                "module export returned a null descriptor".into(),
            ));
        }
        &*export
    };
    validate_export::<H>(manager, export)?;

    let descriptors = std::panic::catch_unwind(|| (export.handlers)())
        .map_err(|_| ModuleError::Validation("handler discovery panicked".into()))?
        .map_err(ModuleError::Validation)?;
    let mut handlers = BTreeMap::new();
    let mut classes = BTreeSet::new();
    for descriptor in descriptors {
        let (class, method) = descriptor.route.split_once('.').ok_or_else(|| {
            ModuleError::Validation(format!("invalid route {}", descriptor.route))
        })?;
        if class.is_empty() || method.is_empty() || method.contains('.') {
            return Err(ModuleError::Validation(format!(
                "invalid route {}",
                descriptor.route
            )));
        }
        if descriptor.package != export.package {
            return Err(ModuleError::Validation(format!(
                "handler {} leaked from dependency package {}",
                descriptor.route, descriptor.package
            )));
        }
        if handlers
            .insert(descriptor.route.to_owned(), descriptor.dispatch)
            .is_some()
        {
            return Err(ModuleError::Validation(format!(
                "duplicate route {}",
                descriptor.route
            )));
        }
        classes.insert(class.to_owned());
    }
    if handlers.is_empty() {
        return Err(ModuleError::Validation(
            "module exports no RPC handlers".into(),
        ));
    }
    let compatibility = ModuleCompatibility {
        model: export.model.map(str::to_owned),
        datecode_min: export.datecode_min.map(str::to_owned),
        datecode_max: export.datecode_max.map(str::to_owned),
    };
    validate_conflicts(manager, export.module_id, source, &classes, &compatibility)?;

    let init = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        (export.init)(&manager.host as *const Arc<H> as *const ())
    }))
    .map_err(|_| ModuleError::Init("module initialization panicked".into()))?;
    let state = tokio::time::timeout(
        manager.options.init_timeout,
        std::panic::AssertUnwindSafe(init).catch_unwind(),
    )
    .await
    .map_err(|_| ModuleError::Timeout("initialization"))?
    .map_err(|_| ModuleError::Init("module initialization panicked".into()))?
    .map_err(ModuleError::Init)?;
    Ok(Arc::new(ModuleGeneration {
        id: export.module_id.to_owned(),
        version: export.module_version.to_owned(),
        source: source.to_owned(),
        shadow,
        hash,
        generation,
        compatibility,
        services: export
            .services
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        classes: classes.into_iter().collect(),
        handlers,
        state,
        shutdown: export.shutdown,
        loaded_at: unix_now(),
        accepting: true.into(),
        missing: false.into(),
        active: 0.into(),
        calls: 0.into(),
        reloads: 0.into(),
        panics: 0.into(),
        last_error: Default::default(),
        _library: library,
    }))
}

fn validate_conflicts<H: ?Sized + Send + Sync + 'static>(
    manager: &ManagerInner<H>,
    module_id: &str,
    source: &Path,
    classes: &BTreeSet<String>,
    compatibility: &ModuleCompatibility,
) -> Result<(), ModuleError> {
    for module in manager.modules.read().unwrap().values() {
        if module.id == module_id && module.source != source {
            return Err(ModuleError::Validation(format!(
                "module id {module_id} is already provided by {}",
                module.source.display()
            )));
        }
        if module.id != module_id
            && module.compatibility.overlaps(compatibility)
            && let Some(class) = classes.iter().find(|class| module.classes.contains(class))
        {
            return Err(ModuleError::Validation(format!(
                "class {class} overlaps caller compatibility with {}",
                module.id
            )));
        }
    }
    Ok(())
}

fn validate_export<H: ?Sized + Send + Sync + 'static>(
    manager: &ManagerInner<H>,
    export: &ModuleExport,
) -> Result<(), ModuleError> {
    if export.magic != MODULE_MAGIC
        || export.abi_version != MODULE_ABI_VERSION
        || export.struct_size != std::mem::size_of::<ModuleExport>()
    {
        return Err(ModuleError::Validation(
            "unsupported module descriptor ABI".into(),
        ));
    }
    if export.vibea3_version != env!("CARGO_PKG_VERSION")
        || (export.build_fingerprint)() != build_fingerprint()
    {
        return Err(ModuleError::Validation(
            "module was built with an incompatible Vibea3/Rust target".into(),
        ));
    }
    if export.module_id.is_empty() || export.module_version.is_empty() {
        return Err(ModuleError::Validation(
            "module id and version must be non-empty".into(),
        ));
    }
    if export.model.is_some_and(|model| {
        model.is_empty()
            || !model.is_ascii()
            || !model
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    }) {
        return Err(ModuleError::Validation(
            "module model must be a non-empty ASCII product identifier".into(),
        ));
    }
    for datecode in [export.datecode_min, export.datecode_max]
        .into_iter()
        .flatten()
    {
        if datecode.len() != DATECODE_DIGIT_COUNT
            || !datecode.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(ModuleError::Validation(
                "module datecodes must contain exactly 8 digits".into(),
            ));
        }
    }
    if export
        .datecode_min
        .zip(export.datecode_max)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(ModuleError::Validation(
            "module datecode_min exceeds datecode_max".into(),
        ));
    }
    let mut services = BTreeSet::new();
    for service in export.services {
        if service.is_empty()
            || !service.is_ascii()
            || !service
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            || !services.insert(*service)
        {
            return Err(ModuleError::Validation(format!(
                "invalid or duplicate module service {service}"
            )));
        }
    }
    if (export.host_type)() != std::any::type_name::<H>() {
        return Err(ModuleError::Validation(format!(
            "host type mismatch: module expects {}, server provides {}",
            (export.host_type)(),
            std::any::type_name::<H>()
        )));
    }
    if export.host_fingerprint != manager.host_fingerprint {
        return Err(ModuleError::Validation(
            "host API fingerprint mismatch".into(),
        ));
    }
    Ok(())
}

async fn stable_file(path: &Path, delay: std::time::Duration) -> Result<(), ModuleError> {
    let first = fingerprint(path)?;
    tokio::time::sleep(delay).await;
    if first != fingerprint(path)? {
        return Err(ModuleError::Validation(format!(
            "module file is still changing: {}",
            path.display()
        )));
    }
    Ok(())
}

fn fingerprint(path: &Path) -> Result<(u64, Option<std::time::SystemTime>), ModuleError> {
    let metadata = std::fs::metadata(path)?;
    Ok((metadata.len(), metadata.modified().ok()))
}

pub(super) fn file_hash(path: &Path) -> Result<String, ModuleError> {
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
