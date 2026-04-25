mod helpers;

use axum::body::{to_bytes, Body};
use axum::http;
use roadboard_core::user::{GrantType, McpTokenRepository, NewMcpToken};
use roadboard_storage::SqliteMcpTokenRepository;
use serde_json::json;
use tower::ServiceExt;

async fn mcp_call(
    app: axum::Router,
    token: &str,
    body: serde_json::Value,
) -> serde_json::Value {
    let req = http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn mcp_e2e_planning_flow() {
    let pool = helpers::setup_db().await;

    // Create user and token with full scopes
    let user_id = helpers::create_test_user(&pool, "agent", "pass").await;
    let created = SqliteMcpTokenRepository(pool.clone())
        .create(NewMcpToken {
            user_id: user_id.clone(),
            token_name: "e2e-token".to_string(),
            scopes: vec![
                GrantType::ProjectAdmin,
                GrantType::ProjectRead,
                GrantType::ProjectWrite,
                GrantType::TaskWrite,
            ],
            expires_at: None,
        })
        .await
        .expect("create token");
    let pt = created.plaintext_token;

    let app = helpers::build_test_app(pool).await;

    // initialize
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "initialize", "params": {}, "id": 1 }),
    )
    .await;
    assert_eq!(res["jsonrpc"], "2.0");
    assert_eq!(res["result"]["protocolVersion"], "2024-11-05");
    assert!(res["error"].is_null());

    // tools/list
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/list", "params": {}, "id": 2 }),
    )
    .await;
    let tools = &res["result"]["tools"];
    assert!(tools.is_array());
    assert_eq!(tools.as_array().unwrap().len(), 16);

    // initial_instructions
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "initial_instructions", "arguments": {} }, "id": 3 }),
    )
    .await;
    assert_eq!(res["result"]["schema_version"], 1);
    assert_eq!(res["result"]["protocol_version"], "roadboard-3.0");

    // create_project
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "create_project", "arguments": { "name": "Test Project" } }, "id": 4 }),
    )
    .await;
    assert!(res["result"]["error"].is_null(), "create_project failed: {res}");
    let project_id = res["result"]["project"]["id"].as_str().unwrap().to_string();
    assert!(!project_id.is_empty());

    // create_milestone
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "create_milestone", "arguments": {
                "projectId": project_id, "title": "M1 Foundation"
            }}, "id": 5 }),
    )
    .await;
    assert!(res["result"]["error"].is_null(), "create_milestone failed: {res}");
    let milestone_id = res["result"]["milestone"]["id"].as_str().unwrap().to_string();

    // create_sprint
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "create_sprint", "arguments": {
                "projectId": project_id,
                "name": "Sprint 1",
                "startDate": "2026-01-01",
                "endDate": "2026-01-14"
            }}, "id": 6 }),
    )
    .await;
    assert!(res["result"]["error"].is_null(), "create_sprint failed: {res}");
    let sprint_id = res["result"]["sprint"]["id"].as_str().unwrap().to_string();

    // create_task
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "create_task", "arguments": {
                "projectId": project_id,
                "title": "Implement auth",
                "milestoneId": milestone_id
            }}, "id": 7 }),
    )
    .await;
    assert!(res["result"]["error"].is_null(), "create_task failed: {res}");
    let task_id = res["result"]["task"]["id"].as_str().unwrap().to_string();

    // add_task_to_sprint
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "add_task_to_sprint", "arguments": {
                "sprintId": sprint_id, "taskId": task_id
            }}, "id": 8 }),
    )
    .await;
    assert!(res["result"]["error"].is_null(), "add_task_to_sprint failed: {res}");
    assert_eq!(res["result"]["sprint_task"]["sprint_id"], sprint_id);

    // update_task_status → done
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "update_task_status", "arguments": {
                "taskId": task_id, "status": "done"
            }}, "id": 9 }),
    )
    .await;
    assert!(res["result"]["error"].is_null(), "update_task_status failed: {res}");
    assert_eq!(res["result"]["task"]["status"], "done");

    // list_tasks with status=done → 1 result
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "list_tasks", "arguments": {
                "projectId": project_id, "status": "done"
            }}, "id": 10 }),
    )
    .await;
    assert!(res["result"]["error"].is_null(), "list_tasks failed: {res}");
    assert_eq!(res["result"]["items"].as_array().unwrap().len(), 1);

    // get_project → check counts
    let res = mcp_call(
        app.clone(),
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "get_project", "arguments": { "projectId": project_id } }, "id": 11 }),
    )
    .await;
    assert!(res["result"]["error"].is_null(), "get_project failed: {res}");
    assert_eq!(res["result"]["counts"]["milestones"], 1);
    assert_eq!(res["result"]["counts"]["tasks"], 1);
}

#[tokio::test]
async fn forbidden_without_task_write_scope() {
    let pool = helpers::setup_db().await;
    let user_id = helpers::create_test_user(&pool, "reader", "pass").await;

    // Token with only project.read — no task.write
    let created = SqliteMcpTokenRepository(pool.clone())
        .create(NewMcpToken {
            user_id,
            token_name: "read-only".to_string(),
            scopes: vec![GrantType::ProjectRead],
            expires_at: None,
        })
        .await
        .expect("create token");
    let pt = created.plaintext_token;

    let app = helpers::build_test_app(pool).await;

    let res = mcp_call(
        app,
        &pt,
        json!({ "jsonrpc": "2.0", "method": "tools/call",
            "params": { "name": "create_task", "arguments": {
                "projectId": "some-id", "title": "Should fail"
            }}, "id": 1 }),
    )
    .await;
    // Tool-level error in result, not HTTP-level error
    assert!(res["error"].is_null(), "should be JSON-RPC result, not JSON-RPC error");
    assert_eq!(res["result"]["error"]["code"], "FORBIDDEN");
}

#[tokio::test]
async fn invalid_bearer_token_returns_401() {
    let pool = helpers::setup_db().await;
    let app = helpers::build_test_app(pool).await;

    let req = http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Authorization", "Bearer notavalidtoken")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({ "jsonrpc": "2.0", "method": "initialize", "params": {}, "id": 1 }).to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn add_task_to_sprint_twice_returns_already_in_sprint() {
    let pool = helpers::setup_db().await;
    let user_id = helpers::create_test_user(&pool, "agent2", "pass").await;
    let created = SqliteMcpTokenRepository(pool.clone())
        .create(NewMcpToken {
            user_id,
            token_name: "full".to_string(),
            scopes: vec![GrantType::ProjectAdmin, GrantType::TaskWrite],
            expires_at: None,
        })
        .await
        .unwrap();
    let pt = created.plaintext_token;
    let app = helpers::build_test_app(pool).await;

    // Create project
    let res = mcp_call(app.clone(), &pt, json!({ "jsonrpc": "2.0", "method": "tools/call",
        "params": { "name": "create_project", "arguments": { "name": "P" } }, "id": 1 })).await;
    let project_id = res["result"]["project"]["id"].as_str().unwrap().to_string();

    // Create sprint
    let res = mcp_call(app.clone(), &pt, json!({ "jsonrpc": "2.0", "method": "tools/call",
        "params": { "name": "create_sprint", "arguments": {
            "projectId": project_id, "name": "S", "startDate": "2026-02-01", "endDate": "2026-02-14"
        }}, "id": 2 })).await;
    let sprint_id = res["result"]["sprint"]["id"].as_str().unwrap().to_string();

    // Create task
    let res = mcp_call(app.clone(), &pt, json!({ "jsonrpc": "2.0", "method": "tools/call",
        "params": { "name": "create_task", "arguments": {
            "projectId": project_id, "title": "T"
        }}, "id": 3 })).await;
    let task_id = res["result"]["task"]["id"].as_str().unwrap().to_string();

    // First add — ok
    let res = mcp_call(app.clone(), &pt, json!({ "jsonrpc": "2.0", "method": "tools/call",
        "params": { "name": "add_task_to_sprint", "arguments": {
            "sprintId": sprint_id, "taskId": task_id
        }}, "id": 4 })).await;
    assert!(res["result"]["error"].is_null(), "first add failed: {res}");

    // Second add — TASK_ALREADY_IN_SPRINT
    let res = mcp_call(app.clone(), &pt, json!({ "jsonrpc": "2.0", "method": "tools/call",
        "params": { "name": "add_task_to_sprint", "arguments": {
            "sprintId": sprint_id, "taskId": task_id
        }}, "id": 5 })).await;
    assert_eq!(res["result"]["error"]["code"], "TASK_ALREADY_IN_SPRINT");
}
