use serde_json::{json, Value};
use uuid::Uuid;

use roadboard_core::user::McpToken;

use crate::state::AppState;
use super::tools;
use super::transport::JsonRpcError;

pub async fn dispatch(
    method: &str,
    params: Option<Value>,
    token: &McpToken,
    state: &AppState,
) -> Result<Value, JsonRpcError> {
    match method {
        "initialize" => Ok(handle_initialize()),
        "tools/list" => Ok(handle_tools_list()),
        "tools/call" => handle_tools_call(params, token, state).await,
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Method not found: {method}"),
            data: None,
        }),
    }
}

fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "roadboard",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

async fn handle_tools_call(
    params: Option<Value>,
    token: &McpToken,
    state: &AppState,
) -> Result<Value, JsonRpcError> {
    let params = params.unwrap_or(Value::Object(Default::default()));

    let tool_name = params["name"].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid params: missing 'name'".to_string(),
        data: None,
    })?;

    let arguments = match params.get("arguments") {
        Some(Value::Null) | None => Value::Object(Default::default()),
        Some(v) => v.clone(),
    };

    let request_id = Uuid::now_v7().to_string();

    let result = dispatch_tool(tool_name, arguments, token, state, &request_id).await;

    Ok(result.unwrap_or_else(|e| {
        if matches!(e, super::error::ToolError::Internal { .. }) {
            tracing::error!(%request_id, tool = tool_name, "internal error in MCP tool");
        }
        e.into_result_value()
    }))
}

async fn dispatch_tool(
    name: &str,
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, super::error::ToolError> {
    match name {
        "initial_instructions" => tools::initial_instructions::run().await,
        "list_projects" => tools::projects::list_projects(args, token, state, request_id).await,
        "get_project" => tools::projects::get_project(args, token, state, request_id).await,
        "create_project" => tools::projects::create_project(args, token, state, request_id).await,
        "list_milestones" => tools::milestones::list_milestones(args, token, state, request_id).await,
        "create_milestone" => tools::milestones::create_milestone(args, token, state, request_id).await,
        "list_sprints" => tools::sprints::list_sprints(args, token, state, request_id).await,
        "get_active_sprint" => tools::sprints::get_active_sprint(args, token, state, request_id).await,
        "create_sprint" => tools::sprints::create_sprint(args, token, state, request_id).await,
        "add_task_to_sprint" => tools::sprints::add_task_to_sprint(args, token, state, request_id).await,
        "remove_task_from_sprint" => tools::sprints::remove_task_from_sprint(args, token, state, request_id).await,
        "list_tasks" => tools::tasks::list_tasks(args, token, state, request_id).await,
        "get_task" => tools::tasks::get_task(args, token, state, request_id).await,
        "create_task" => tools::tasks::create_task(args, token, state, request_id).await,
        "update_task" => tools::tasks::update_task(args, token, state, request_id).await,
        "update_task_status" => tools::tasks::update_task_status(args, token, state, request_id).await,
        _ => Err(super::error::ToolError::NotFound {
            entity_type: "Tool",
            entity_id: name.to_string(),
        }),
    }
}

fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "initial_instructions",
                "description": "Returns operational rules, tool registry, and error code summary for this session",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "list_projects",
                "description": "List projects accessible to the caller",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "cursor": { "type": "string", "description": "Opaque pagination cursor" },
                        "limit": { "type": "integer", "description": "Max results (1-200, default 50)" }
                    }
                }
            },
            {
                "name": "get_project",
                "description": "Get project detail with milestone, sprint, task and member counts",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "projectId": { "type": "string" }
                    },
                    "required": ["projectId"]
                }
            },
            {
                "name": "create_project",
                "description": "Create a new project; caller is automatically added as Owner",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "slug": { "type": "string", "description": "URL-safe identifier; auto-generated from name if omitted" },
                        "description": { "type": "string" }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "list_milestones",
                "description": "List milestones for a project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "projectId": { "type": "string" },
                        "status": { "type": "string", "enum": ["planned", "in_progress", "done", "cancelled"] },
                        "cursor": { "type": "string" },
                        "limit": { "type": "integer" }
                    },
                    "required": ["projectId"]
                }
            },
            {
                "name": "create_milestone",
                "description": "Create a milestone in a project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "projectId": { "type": "string" },
                        "title": { "type": "string" },
                        "dueDate": { "type": "string", "description": "ISO-8601 date (YYYY-MM-DD)" },
                        "description": { "type": "string" },
                        "orderIndex": { "type": "integer" }
                    },
                    "required": ["projectId", "title"]
                }
            },
            {
                "name": "list_sprints",
                "description": "List sprints for a project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "projectId": { "type": "string" },
                        "status": { "type": "string", "enum": ["planned", "active", "closed"] },
                        "cursor": { "type": "string" },
                        "limit": { "type": "integer" }
                    },
                    "required": ["projectId"]
                }
            },
            {
                "name": "get_active_sprint",
                "description": "Return the active sprint for a project, or null if none",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "projectId": { "type": "string" }
                    },
                    "required": ["projectId"]
                }
            },
            {
                "name": "create_sprint",
                "description": "Create a sprint in a project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "projectId": { "type": "string" },
                        "name": { "type": "string" },
                        "startDate": { "type": "string", "description": "ISO-8601 date (YYYY-MM-DD)" },
                        "endDate": { "type": "string", "description": "ISO-8601 date (YYYY-MM-DD)" },
                        "goal": { "type": "string" }
                    },
                    "required": ["projectId", "name", "startDate", "endDate"]
                }
            },
            {
                "name": "add_task_to_sprint",
                "description": "Link a task to a sprint",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sprintId": { "type": "string" },
                        "taskId": { "type": "string" },
                        "carriedFromSprintId": { "type": "string" }
                    },
                    "required": ["sprintId", "taskId"]
                }
            },
            {
                "name": "remove_task_from_sprint",
                "description": "Soft-remove a task from a sprint",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sprintId": { "type": "string" },
                        "taskId": { "type": "string" }
                    },
                    "required": ["sprintId", "taskId"]
                }
            },
            {
                "name": "list_tasks",
                "description": "List tasks with optional filters",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "projectId": { "type": "string" },
                        "status": { "type": "string", "enum": ["todo", "in_progress", "blocked", "done", "cancelled"] },
                        "milestoneId": { "type": "string" },
                        "sprintId": { "type": "string" },
                        "assigneeUserId": { "type": "string" },
                        "cursor": { "type": "string" },
                        "limit": { "type": "integer" }
                    },
                    "required": ["projectId"]
                }
            },
            {
                "name": "get_task",
                "description": "Get a single task by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "taskId": { "type": "string" }
                    },
                    "required": ["taskId"]
                }
            },
            {
                "name": "create_task",
                "description": "Create a task in a project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "projectId": { "type": "string" },
                        "title": { "type": "string" },
                        "milestoneId": { "type": "string" },
                        "sprintId": { "type": "string" },
                        "description": { "type": "string" },
                        "priority": { "type": "string", "enum": ["low", "medium", "high", "urgent"] },
                        "assigneeUserId": { "type": "string" },
                        "dueDate": { "type": "string", "description": "ISO-8601 date (YYYY-MM-DD)" },
                        "estimate": { "type": "string" }
                    },
                    "required": ["projectId", "title"]
                }
            },
            {
                "name": "update_task",
                "description": "Update task fields (partial update)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "taskId": { "type": "string" },
                        "title": { "type": "string" },
                        "description": { "type": "string" },
                        "milestoneId": { "type": "string" },
                        "priority": { "type": "string", "enum": ["low", "medium", "high", "urgent"] },
                        "assigneeUserId": { "type": "string" },
                        "dueDate": { "type": "string" },
                        "estimate": { "type": "string" }
                    },
                    "required": ["taskId"]
                }
            },
            {
                "name": "update_task_status",
                "description": "Transition task status",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "taskId": { "type": "string" },
                        "status": { "type": "string", "enum": ["todo", "in_progress", "blocked", "done", "cancelled"] },
                        "completionReport": { "type": "string" }
                    },
                    "required": ["taskId", "status"]
                }
            }
        ]
    })
}
