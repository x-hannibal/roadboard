mod helpers;

use axum::body::{to_bytes, Body};
use axum::http;
use tower::ServiceExt;

#[tokio::test]
async fn post_without_csrf_token_returns_403() {
    let pool = helpers::setup_db().await;
    let app = helpers::build_test_app(pool).await;

    // GET request to obtain the csrf_token cookie
    let get_req = http::Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .body(Body::empty())
        .unwrap();
    let get_resp = app.clone().oneshot(get_req).await.unwrap();
    let set_cookies = helpers::extract_set_cookies(&get_resp);
    let csrf_cookie = set_cookies
        .iter()
        .find(|c| c.starts_with("csrf_token="))
        .cloned()
        .unwrap_or_default();
    assert!(!csrf_cookie.is_empty(), "csrf_token cookie should be set on GET");

    // POST /api/projects WITHOUT X-CSRF-Token header — should return 403
    let proj_body = serde_json::json!({ "slug": "test", "name": "Test" });
    let post_req = http::Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("Content-Type", "application/json")
        .header("Cookie", &csrf_cookie)
        // deliberately omit X-CSRF-Token
        .body(Body::from(proj_body.to_string()))
        .unwrap();

    let post_resp = app.clone().oneshot(post_req).await.unwrap();
    assert_eq!(post_resp.status(), 403);
    let bytes = to_bytes(post_resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"]["code"], "FORBIDDEN");
}

#[tokio::test]
async fn post_with_correct_csrf_token_passes_csrf_check() {
    let pool = helpers::setup_db().await;
    let app = helpers::build_test_app(pool).await;

    // GET to get csrf cookie
    let get_req = http::Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .body(Body::empty())
        .unwrap();
    let get_resp = app.clone().oneshot(get_req).await.unwrap();
    let set_cookies = helpers::extract_set_cookies(&get_resp);
    let csrf_cookie = set_cookies
        .iter()
        .find(|c| c.starts_with("csrf_token="))
        .cloned()
        .unwrap_or_default();
    let csrf_token = csrf_cookie.strip_prefix("csrf_token=").unwrap_or("").to_string();

    // POST /api/projects WITH correct X-CSRF-Token — CSRF check passes
    // (request may fail with 401 because no session, but NOT 403 from CSRF)
    let proj_body = serde_json::json!({ "slug": "test", "name": "Test" });
    let post_req = http::Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("Content-Type", "application/json")
        .header("Cookie", &csrf_cookie)
        .header("X-CSRF-Token", &csrf_token)
        .body(Body::from(proj_body.to_string()))
        .unwrap();

    let post_resp = app.clone().oneshot(post_req).await.unwrap();
    // CSRF check passed — status is 401 (no auth), not 403 (CSRF)
    assert_ne!(post_resp.status(), 403, "CSRF check should not reject a valid token");
    assert_eq!(post_resp.status(), 401, "Should be 401 because not logged in");
}

#[tokio::test]
async fn login_endpoint_is_exempt_from_csrf() {
    let pool = helpers::setup_db().await;
    helpers::create_test_user(&pool, "exemptuser", "pass").await;
    let app = helpers::build_test_app(pool).await;

    // POST /api/auth/login WITHOUT any CSRF token/cookie — should NOT be rejected by CSRF
    let body = serde_json::json!({ "username": "exemptuser", "password": "pass" });
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("Content-Type", "application/json")
        // no Cookie, no X-CSRF-Token
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // Should return 200 (login succeeds), not 403 (CSRF rejection)
    assert_eq!(resp.status(), 200);
}
