mod helpers;

use axum::body::{to_bytes, Body};
use axum::http;
use tower::ServiceExt;

#[tokio::test]
async fn login_correct_credentials_returns_200_with_user() {
    let pool = helpers::setup_db().await;
    helpers::create_test_user(&pool, "alice", "s3cret").await;
    let app = helpers::build_test_app(pool).await;

    let body = serde_json::json!({ "username": "alice", "password": "s3cret" });
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["user"]["username"], "alice");
}

#[tokio::test]
async fn login_wrong_password_returns_401() {
    let pool = helpers::setup_db().await;
    helpers::create_test_user(&pool, "bob", "correct").await;
    let app = helpers::build_test_app(pool).await;

    let body = serde_json::json!({ "username": "bob", "password": "wrong" });
    let req = http::Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 401);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"]["code"], "UNAUTHENTICATED");
}

#[tokio::test]
async fn me_without_session_returns_401() {
    let pool = helpers::setup_db().await;
    let app = helpers::build_test_app(pool).await;

    let req = http::Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn me_with_session_returns_user_data() {
    let pool = helpers::setup_db().await;
    helpers::create_test_user(&pool, "carol", "pw123").await;
    let app = helpers::build_test_app(pool).await;

    // Login — get session cookie
    let body = serde_json::json!({ "username": "carol", "password": "pw123" });
    let login_req = http::Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let login_resp = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_resp.status(), 200);
    let cookies = helpers::extract_set_cookies(&login_resp);
    let session_cookie = cookies.iter().find(|c| c.starts_with("id=")).unwrap();

    // Call /me with the session cookie
    let me_req = http::Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .header("Cookie", session_cookie)
        .body(Body::empty())
        .unwrap();

    let me_resp = app.clone().oneshot(me_req).await.unwrap();
    assert_eq!(me_resp.status(), 200);

    let bytes = to_bytes(me_resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["user"]["username"], "carol");
}

#[tokio::test]
async fn logout_clears_session() {
    let pool = helpers::setup_db().await;
    helpers::create_test_user(&pool, "dave", "pw456").await;
    let app = helpers::build_test_app(pool).await;

    // Login
    let body = serde_json::json!({ "username": "dave", "password": "pw456" });
    let login_req = http::Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let login_resp = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_resp.status(), 200);
    let cookies = helpers::extract_set_cookies(&login_resp);
    let session_cookie = cookies.iter().find(|c| c.starts_with("id=")).unwrap().clone();

    // Get CSRF token (GET request generates one)
    let csrf_req = http::Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .header("Cookie", &session_cookie)
        .body(Body::empty())
        .unwrap();
    let csrf_resp = app.clone().oneshot(csrf_req).await.unwrap();
    let all_cookies = helpers::extract_set_cookies(&csrf_resp);
    let csrf_cookie = all_cookies
        .iter()
        .find(|c| c.starts_with("csrf_token="))
        .cloned()
        .unwrap_or_default();
    let csrf_token = csrf_cookie.strip_prefix("csrf_token=").unwrap_or("");

    // Logout
    let combined_cookies = format!("{}; {}", session_cookie, csrf_cookie);
    let logout_req = http::Request::builder()
        .method("POST")
        .uri("/api/auth/logout")
        .header("Cookie", &combined_cookies)
        .header("X-CSRF-Token", csrf_token)
        .body(Body::empty())
        .unwrap();

    let logout_resp = app.clone().oneshot(logout_req).await.unwrap();
    assert_eq!(logout_resp.status(), 204);

    // /me after logout should be 401
    let me_req = http::Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .header("Cookie", &session_cookie)
        .body(Body::empty())
        .unwrap();

    let me_resp = app.clone().oneshot(me_req).await.unwrap();
    assert_eq!(me_resp.status(), 401);
}
