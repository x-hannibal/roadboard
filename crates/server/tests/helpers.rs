#![allow(dead_code)]

use axum::{body::Body, http};
use roadboard_core::user::{NewUser, UserRepository};
use roadboard_server::AppState;
use roadboard_storage::{migrate, SqliteUserRepository};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tower::ServiceExt;

pub async fn setup_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory connect");
    migrate(&pool).await.expect("migrate");
    pool
}

pub async fn build_test_app(pool: SqlitePool) -> axum::Router {
    let state = AppState { pool };
    roadboard_server::build_router(state).await.expect("build router")
}

pub async fn create_test_user(pool: &SqlitePool, username: &str, password: &str) -> String {
    SqliteUserRepository(pool.clone())
        .create(NewUser {
            username: username.to_string(),
            email: format!("{}@test.com", username),
            display_name: username.to_string(),
            // storage layer hashes this
            password_hash: password.to_string(),
        })
        .await
        .expect("create user")
        .id
}

pub async fn login(app: axum::Router, username: &str, password: &str) -> (String, String) {
    let body = serde_json::json!({
        "username": username,
        "password": password
    });
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), 200, "login should succeed");

    let cookies = extract_set_cookies(&response);
    let session_cookie = cookies
        .iter()
        .find(|c| c.starts_with("id="))
        .expect("session cookie")
        .clone();
    let csrf_cookie = cookies
        .iter()
        .find(|c| c.starts_with("csrf_token="))
        .cloned()
        .unwrap_or_default();

    let csrf_token = csrf_cookie
        .strip_prefix("csrf_token=")
        .unwrap_or("")
        .to_string();

    (session_cookie, csrf_token)
}

pub fn extract_set_cookies(resp: &http::Response<Body>) -> Vec<String> {
    resp.headers()
        .get_all(http::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|v| {
            // Extract "name=value" (strip flags like "; HttpOnly; Path=/ ...")
            v.split(';').next().unwrap_or("").trim().to_string()
        })
        .filter(|v| !v.is_empty())
        .collect()
}

pub fn cookie_header(cookies: &[&str]) -> String {
    cookies.join("; ")
}
