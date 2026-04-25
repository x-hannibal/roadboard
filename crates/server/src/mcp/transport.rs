use axum::{
    extract::{Extension, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use roadboard_core::user::McpToken;

use crate::state::AppState;
use super::dispatcher;

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Value,
}

#[derive(Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

#[derive(Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub async fn mcp_post(
    State(state): State<AppState>,
    Extension(token): Extension<McpToken>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    if req.jsonrpc != "2.0" {
        return Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid Request: jsonrpc must be \"2.0\"".to_string(),
                data: None,
            }),
            id: req.id,
        });
    }

    let (result, error) =
        match dispatcher::dispatch(&req.method, req.params, &token, &state).await {
            Ok(val) => (Some(val), None),
            Err(err) => (None, Some(err)),
        };

    Json(JsonRpcResponse { jsonrpc: "2.0".to_string(), result, error, id: req.id })
}

pub async fn mcp_get(Extension(_token): Extension<McpToken>) -> impl IntoResponse {
    (
        [("Content-Type", "text/event-stream"), ("Cache-Control", "no-cache")],
        "event: ping\ndata: {}\n\n",
    )
}
