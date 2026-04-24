use anyhow::{Context, Result};
use std::process::Command;

pub fn run_command(
    server: bool,
    _with_web: bool,
    _port: Option<u16>,
    log_level: &str,
) -> Result<()> {
    if server {
        // TODO: use cargo-watch for hot reload when available
        let status = Command::new("cargo")
            .args(["run", "-p", "roadboard-server", "--", "serve"])
            .env("RUST_LOG", log_level)
            .status()
            .context("failed to spawn cargo — is Rust installed?")?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    } else {
        println!("Tauri scaffolding lands in M5. Use `robot run --server` for Axum-only mode.");
        Ok(())
    }
}
