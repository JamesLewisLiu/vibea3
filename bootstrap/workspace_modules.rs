use std::{
    collections::HashSet,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Deserialize;
use tracing::info;

use crate::AnyError;

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    target_directory: PathBuf,
    workspace_root: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    targets: Vec<Target>,
    metadata: Option<PackageMetadata>,
}

#[derive(Default, Deserialize)]
struct PackageMetadata {
    #[serde(default)]
    vibea3: Vibea3Metadata,
}

#[derive(Default, Deserialize)]
struct Vibea3Metadata {
    #[serde(default)]
    module: bool,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    crate_types: Vec<String>,
}

struct Module {
    package: String,
    library: String,
}

pub(crate) fn build_and_deploy(directory: &Path, release: bool) -> Result<(), AnyError> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let metadata = metadata(&cargo)?;
    let modules = deployable_modules(&metadata);
    if modules.is_empty() {
        return Err("workspace contains no deployable Vibea3 modules".into());
    }

    info!(
        modules = modules.len(),
        release,
        destination = %directory.display(),
        "building workspace XRPC modules"
    );
    let mut build = Command::new(&cargo);
    build.arg("build");
    for module in &modules {
        build.args(["--package", &module.package]);
    }
    if release {
        build.arg("--release");
    }
    let status = build.current_dir(&metadata.workspace_root).status()?;
    if !status.success() {
        return Err(format!("module build failed with {status}").into());
    }

    fs::create_dir_all(directory)?;
    let profile = if release { "release" } else { "debug" };
    for module in modules {
        let filename = library_filename(&module.library);
        let source = metadata.target_directory.join(profile).join(&filename);
        let destination = directory.join(&filename);
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "failed to deploy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
        info!(
            package = %module.package,
            path = %destination.display(),
            "workspace XRPC module deployed"
        );
    }
    Ok(())
}

fn metadata(cargo: &OsString) -> Result<Metadata, AnyError> {
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn deployable_modules(metadata: &Metadata) -> Vec<Module> {
    let members: HashSet<_> = metadata.workspace_members.iter().collect();
    let mut modules: Vec<_> = metadata
        .packages
        .iter()
        .filter(|package| {
            members.contains(&package.id)
                && package
                    .metadata
                    .as_ref()
                    .is_some_and(|metadata| metadata.vibea3.module)
        })
        .flat_map(|package| {
            package
                .targets
                .iter()
                .filter(|target| target.crate_types.iter().any(|kind| kind == "cdylib"))
                .map(|target| Module {
                    package: package.name.clone(),
                    library: target.name.clone(),
                })
        })
        .collect();
    modules.sort_unstable_by(|left, right| left.package.cmp(&right.package));
    modules
}

fn library_filename(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{name}.dylib")
    } else {
        format!("lib{name}.so")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_builds_and_deploys_only_production_modules() {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
        let metadata = metadata(&cargo).unwrap();
        let modules = deployable_modules(&metadata);
        assert_eq!(
            modules
                .iter()
                .map(|module| module.package.as_str())
                .collect::<Vec<_>>(),
            ["core", "popn_highcheers", "sdvx_nabula"]
        );

        let directory = tempfile::tempdir().unwrap();
        build_and_deploy(directory.path(), false).unwrap();
        assert!(directory.path().join(library_filename("core")).is_file());
        assert!(
            directory
                .path()
                .join(library_filename("popn_highcheers"))
                .is_file()
        );
        assert!(
            !directory
                .path()
                .join(library_filename("vibea3_test_module"))
                .exists()
        );
    }
}
