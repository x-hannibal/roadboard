//! Roadboard HTTP + MCP server library.
//!
//! Both the standalone binary and the Tauri desktop shell embed this library.
//! All business logic lives here; `main.rs` is a thin CLI dispatcher.

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;

/// Start the Axum server.
///
/// Bind address is read from `ROADBOARD_BIND` env var (default `127.0.0.1:8787`).
pub async fn run() -> anyhow::Result<()> {
    let bind: SocketAddr = std::env::var("ROADBOARD_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
        .parse()?;

    let app = build_router();

    tracing::info!("listening on {bind}");
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Build the Axum router. Exported so integration tests can construct it directly.
pub fn build_router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
