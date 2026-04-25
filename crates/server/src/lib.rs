pub mod error;
pub mod state;
mod handlers;
mod mcp;
mod middleware;

pub use state::AppState;

use axum::{
    middleware as axum_middleware,
    routing::{delete, get, patch, post},
    Json, Router,
};
use handlers::{auth, milestones, projects, sprints, tasks, tokens, users};
use mcp::transport::{mcp_get, mcp_post};
use serde_json::json;
use std::net::SocketAddr;
use time::Duration;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

pub async fn build_router(state: AppState) -> anyhow::Result<Router> {
    // Session store backed by the same SQLite pool
    let session_store = SqliteStore::new(state.pool.clone());
    session_store.migrate().await?;

    let insecure = std::env::var("ROADBOARD_INSECURE_COOKIES")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(!insecure)
        .with_expiry(Expiry::OnInactivity(Duration::days(7)));

    // /mcp — bearer auth enforced; POST = JSON-RPC dispatcher, GET = SSE stub
    let mcp_router = Router::new()
        .route("/", post(mcp_post).get(mcp_get))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::bearer_auth_middleware,
        ));

    // Auth routes — no CSRF (login is unauthenticated; logout exempted to keep it simple)
    let auth_router = Router::new()
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/me", get(auth::me));

    // User routes
    let user_router = Router::new()
        .route("/{id}", get(users::get_user))
        .route("/{id}/status", patch(users::update_user_status));

    // Token routes
    let token_router = Router::new()
        .route("/", get(tokens::list_tokens).post(tokens::create_token))
        .route("/{id}", delete(tokens::revoke_token));

    // Project routes
    let project_router = Router::new()
        .route("/", get(projects::list_projects).post(projects::create_project))
        .route("/{id}", get(projects::get_project))
        .route("/{project_id}/members", get(projects::list_members).post(projects::add_member))
        .route("/{project_id}/members/{user_id}", delete(projects::remove_member))
        .route("/{project_id}/milestones", get(milestones::list_milestones).post(milestones::create_milestone))
        .route("/{project_id}/sprints", get(sprints::list_sprints).post(sprints::create_sprint))
        .route("/{project_id}/sprints/active", get(sprints::get_active_sprint))
        .route("/{project_id}/tasks", get(tasks::list_tasks).post(tasks::create_task));

    // Standalone resource routes (by ID)
    let milestone_router = Router::new()
        .route("/{id}", get(milestones::get_milestone));

    let sprint_router = Router::new()
        .route("/{id}", get(sprints::get_sprint));

    let task_router = Router::new()
        .route("/{id}", get(tasks::get_task).patch(tasks::update_task))
        .route("/{id}/status", patch(tasks::update_task_status));

    let sprint_task_router = Router::new()
        .route("/{sprint_id}/tasks", post(sprints::add_task_to_sprint))
        .route("/{sprint_id}/tasks/{task_id}", delete(sprints::remove_task_from_sprint));

    // Assemble /api with CSRF middleware
    let api_router = Router::new()
        .nest("/auth", auth_router)
        .nest("/users", user_router)
        .nest("/tokens", token_router)
        .nest("/projects", project_router)
        .nest("/milestones", milestone_router)
        .nest("/sprints", sprint_router.merge(sprint_task_router))
        .nest("/tasks", task_router)
        .layer(axum_middleware::from_fn(middleware::csrf_middleware));

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api", api_router)
        .nest("/mcp", mcp_router)
        .with_state(state)
        .layer(session_layer);

    Ok(app)
}

pub async fn run() -> anyhow::Result<()> {
    let bind: SocketAddr = std::env::var("ROADBOARD_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
        .parse()?;

    let db_path = std::env::var("ROADBOARD_DB_PATH")
        .unwrap_or_else(|_| "./data/roadboard.db".to_string());

    let pool = roadboard_storage::connect(&db_path).await?;

    let auto_migrate = std::env::var("ROADBOARD_AUTO_MIGRATE")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    if auto_migrate {
        roadboard_storage::migrate(&pool).await?;
    }

    let state = AppState { pool };
    let app = build_router(state).await?;

    tracing::info!("listening on {bind}");
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
