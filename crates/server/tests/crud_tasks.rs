mod helpers;

use axum::body::{to_bytes, Body};
use axum::http;
use tower::ServiceExt;

#[tokio::test]
async fn create_update_list_tasks_cycle() {
    let pool = helpers::setup_db().await;
    let user_id = helpers::create_test_user(&pool, "testuser", "pass1234").await;
    let app = helpers::build_test_app(pool).await;

    // Login
    let login_body = serde_json::json!({ "username": "testuser", "password": "pass1234" });
    let login_req = http::Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();
    let login_resp = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_resp.status(), 200);

    let cookies = helpers::extract_set_cookies(&login_resp);
    let session_cookie = cookies.iter().find(|c| c.starts_with("id=")).unwrap().clone();

    // Get CSRF token
    let get_req = http::Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .header("Cookie", &session_cookie)
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
    let combined = format!("{}; {}", session_cookie, csrf_cookie);

    // Create project
    let proj_body = serde_json::json!({ "slug": "myproj", "name": "My Project" });
    let proj_req = http::Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("Content-Type", "application/json")
        .header("Cookie", &combined)
        .header("X-CSRF-Token", &csrf_token)
        .body(Body::from(proj_body.to_string()))
        .unwrap();
    let proj_resp = app.clone().oneshot(proj_req).await.unwrap();
    assert_eq!(proj_resp.status(), 201);
    let bytes = to_bytes(proj_resp.into_body(), usize::MAX).await.unwrap();
    let proj_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let project_id = proj_json["project"]["id"].as_str().unwrap().to_string();

    // Create task
    let task_body = serde_json::json!({ "title": "First Task" });
    let task_req = http::Request::builder()
        .method("POST")
        .uri(format!("/api/projects/{}/tasks", project_id))
        .header("Content-Type", "application/json")
        .header("Cookie", &combined)
        .header("X-CSRF-Token", &csrf_token)
        .body(Body::from(task_body.to_string()))
        .unwrap();
    let task_resp = app.clone().oneshot(task_req).await.unwrap();
    assert_eq!(task_resp.status(), 201);
    let bytes = to_bytes(task_resp.into_body(), usize::MAX).await.unwrap();
    let task_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let task_id = task_json["task"]["id"].as_str().unwrap().to_string();
    assert_eq!(task_json["task"]["status"], "todo");

    // List tasks — should have 1
    let list_req = http::Request::builder()
        .method("GET")
        .uri(format!("/api/projects/{}/tasks", project_id))
        .header("Cookie", &combined)
        .body(Body::empty())
        .unwrap();
    let list_resp = app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(list_resp.status(), 200);
    let bytes = to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
    let list_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(list_json["items"].as_array().unwrap().len(), 1);

    // Update task title
    let patch_body = serde_json::json!({ "title": "Renamed Task" });
    let patch_req = http::Request::builder()
        .method("PATCH")
        .uri(format!("/api/tasks/{}", task_id))
        .header("Content-Type", "application/json")
        .header("Cookie", &combined)
        .header("X-CSRF-Token", &csrf_token)
        .body(Body::from(patch_body.to_string()))
        .unwrap();
    let patch_resp = app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(patch_resp.status(), 200);
    let bytes = to_bytes(patch_resp.into_body(), usize::MAX).await.unwrap();
    let patched: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(patched["task"]["title"], "Renamed Task");

    // Update status to done
    let status_body = serde_json::json!({ "status": "done" });
    let status_req = http::Request::builder()
        .method("PATCH")
        .uri(format!("/api/tasks/{}/status", task_id))
        .header("Content-Type", "application/json")
        .header("Cookie", &combined)
        .header("X-CSRF-Token", &csrf_token)
        .body(Body::from(status_body.to_string()))
        .unwrap();
    let status_resp = app.clone().oneshot(status_req).await.unwrap();
    assert_eq!(status_resp.status(), 200);
    let bytes = to_bytes(status_resp.into_body(), usize::MAX).await.unwrap();
    let done_task: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(done_task["task"]["status"], "done");
    assert!(done_task["task"]["completed_at"].is_string());

    // GET task — verify completed_at is set
    let get_task_req = http::Request::builder()
        .method("GET")
        .uri(format!("/api/tasks/{}", task_id))
        .header("Cookie", &combined)
        .body(Body::empty())
        .unwrap();
    let get_task_resp = app.clone().oneshot(get_task_req).await.unwrap();
    assert_eq!(get_task_resp.status(), 200);
    let bytes = to_bytes(get_task_resp.into_body(), usize::MAX).await.unwrap();
    let fetched: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(fetched["task"]["completed_at"].is_string());

    // Suppress unused variable warning
    let _ = user_id;
}
