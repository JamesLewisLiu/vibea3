use std::process::Command;

fn main() {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let version = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "rustc unknown".into());
    println!("cargo:rustc-env=VIBEA3_RUSTC_VERSION={}", version.trim());
    println!(
        "cargo:rustc-env=VIBEA3_TARGET={}",
        std::env::var("TARGET").unwrap_or_else(|_| "unknown-target".into())
    );
    println!("cargo:rerun-if-changed=build.rs");
}
