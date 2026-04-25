use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use roadboard_core::user::{McpTokenRepository, TokenStatus};
use roadboard_storage::SqliteMcpTokenRepository;
use serde_json::json;

use crate::state::AppState;

pub async fn csrf_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let cookie_str = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let csrf_from_cookie = extract_cookie_value(&cookie_str, "csrf_token");

    let is_mutation = matches!(
        method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    // login exempt: user has no token yet; the path is stripped of /api prefix
    let is_exempt = method == Method::POST && path.ends_with("/auth/login");

    if is_mutation && !is_exempt {
        let header_token = req
            .headers()
            .get("X-CSRF-Token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let valid = matches!(
            (&csrf_from_cookie, &header_token),
            (Some(c), Some(h)) if c == h
        );

        if !valid {
            let body = json!({
                "error": { "code": "FORBIDDEN", "message": "CSRF token mismatch" }
            });
            return (StatusCode::FORBIDDEN, Json(body)).into_response();
        }
    }

    let needs_new_token = csrf_from_cookie.is_none();
    let new_token = needs_new_token.then(generate_csrf_token);

    let mut response = next.run(req).await;

    if let Some(token) = new_token {
        let cookie = format!("csrf_token={}; Path=/; SameSite=Lax", token);
        if let Ok(val) = HeaderValue::from_str(&cookie) {
            response.headers_mut().append(header::SET_COOKIE, val);
        }
    }

    response
}

pub async fn bearer_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let token_str = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|v| v.to_string());

    let token_str = match token_str {
        Some(t) => t,
        None => return unauth_response("missing_token"),
    };

    let repo = SqliteMcpTokenRepository(state.pool.clone());
    let token = match repo.find_by_hash(&token_str).await {
        Ok(t) => t,
        Err(roadboard_core::Error::NotFound { .. }) => return unauth_response("invalid_format"),
        Err(_) => {
            let body =
                json!({ "error": { "code": "INTERNAL_ERROR", "message": "Internal error" } });
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response();
        }
    };

    if token.status != TokenStatus::Active {
        return unauth_response("revoked");
    }

    if let Some(expires_at) = token.expires_at {
        if expires_at < Utc::now() {
            return unauth_response("expired");
        }
    }

    let pool = state.pool.clone();
    let token_id = token.id.clone();
    tokio::spawn(async move {
        let _ = SqliteMcpTokenRepository(pool).touch_last_used(&token_id).await;
    });

    req.extensions_mut().insert(token);
    next.run(req).await
}

fn generate_csrf_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn extract_cookie_value(header: &str, name: &str) -> Option<String> {
    let prefix = format!("{}=", name);
    header.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix(&prefix).map(|v| v.to_string())
    })
}

fn unauth_response(reason: &'static str) -> Response {
    let body = json!({
        "error": {
            "code": "UNAUTHENTICATED",
            "message": "Authentication required",
            "details": { "reason": reason }
        }
    });
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}
