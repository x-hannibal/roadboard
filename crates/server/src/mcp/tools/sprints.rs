use chrono::NaiveDate;
use roadboard_core::{
    planning::{
        NewSprint, NewSprintTask, SprintRepository, SprintStatus, SprintTaskRepository,
        TaskRepository,
    },
    user::{GrantType, McpToken},
};
use roadboard_storage::{SqliteSprintRepository, SqliteSprintTaskRepository, SqliteTaskRepository};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::state::AppState;
use super::super::error::ToolError;
use super::{pagination, scope};

pub async fn list_sprints(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectRead)?;
    let project_id = require_str(&args, "projectId")?;
    scope::check_project_access(state, token, project_id, request_id).await?;

    let status = parse_sprint_status(&args)?;
    let limit = pagination::parse_limit(&args)?;
    let after_id = pagination::parse_cursor(&args)?;

    let sprints = SqliteSprintRepository(state.pool.clone())
        .list_for_project(project_id, status, after_id.as_deref(), limit)
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    let items: Vec<Value> =
        sprints.iter().map(|s| serde_json::to_value(s).unwrap_or(Value::Null)).collect();

    Ok(pagination::build_page(items, limit))
}

pub async fn get_active_sprint(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectRead)?;
    let project_id = require_str(&args, "projectId")?;
    scope::check_project_access(state, token, project_id, request_id).await?;

    let sprint = SqliteSprintRepository(state.pool.clone())
        .find_active_for_project(project_id)
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    Ok(json!({ "schema_version": 1, "sprint": sprint }))
}

pub async fn create_sprint(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectWrite)?;
    let project_id = require_str(&args, "projectId")?;
    scope::check_project_access(state, token, project_id, request_id).await?;

    let name = require_str(&args, "name")?;
    let start_date = parse_date(&args, "startDate")?;
    let end_date = parse_date(&args, "endDate")?;

    if end_date <= start_date {
        let mut fields = HashMap::new();
        fields.insert("endDate".to_string(), "must be after startDate".to_string());
        return Err(ToolError::ValidationError { fields });
    }

    let goal = args["goal"].as_str().map(str::to_string);

    let sprint = SqliteSprintRepository(state.pool.clone())
        .create(NewSprint {
            project_id: project_id.to_string(),
            name: name.to_string(),
            goal,
            start_date,
            end_date,
            status: SprintStatus::Planned,
            created_by_user_id: token.user_id.clone(),
            updated_by_user_id: token.user_id.clone(),
        })
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    Ok(json!({ "schema_version": 1, "sprint": sprint }))
}

pub async fn add_task_to_sprint(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::TaskWrite)?;

    let sprint_id = require_str(&args, "sprintId")?;
    let task_id = require_str(&args, "taskId")?;
    let carried_from = args["carriedFromSprintId"].as_str().map(str::to_string);

    // Look up sprint to verify project access and get project_id
    let sprint = SqliteSprintRepository(state.pool.clone())
        .find_by_id(sprint_id)
        .await
        .map_err(|e| match e {
            roadboard_core::Error::NotFound { .. } => {
                ToolError::NotFound { entity_type: "Sprint", entity_id: sprint_id.to_string() }
            }
            other => ToolError::from_core(other, request_id),
        })?;

    scope::check_project_access(state, token, &sprint.project_id, request_id).await?;

    // Look up task to check project_id match
    let task = SqliteTaskRepository(state.pool.clone())
        .find_by_id(task_id)
        .await
        .map_err(|e| match e {
            roadboard_core::Error::NotFound { .. } => {
                ToolError::NotFound { entity_type: "Task", entity_id: task_id.to_string() }
            }
            other => ToolError::from_core(other, request_id),
        })?;

    if task.project_id != sprint.project_id {
        return Err(ToolError::TaskProjectMismatch);
    }

    // Check for existing active link
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sprint_tasks WHERE sprint_id = ? AND task_id = ? AND removed_at IS NULL",
    )
    .bind(sprint_id)
    .bind(task_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ToolError::Internal { request_id: request_id.to_string() })?;

    if active > 0 {
        return Err(ToolError::TaskAlreadyInSprint);
    }

    let sprint_task = SqliteSprintTaskRepository(state.pool.clone())
        .add(NewSprintTask {
            sprint_id: sprint_id.to_string(),
            task_id: task_id.to_string(),
            carried_from_sprint_id: carried_from,
        })
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    Ok(json!({ "schema_version": 1, "sprint_task": sprint_task }))
}

pub async fn remove_task_from_sprint(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::TaskWrite)?;

    let sprint_id = require_str(&args, "sprintId")?;
    let task_id = require_str(&args, "taskId")?;

    let sprint = SqliteSprintRepository(state.pool.clone())
        .find_by_id(sprint_id)
        .await
        .map_err(|e| match e {
            roadboard_core::Error::NotFound { .. } => {
                ToolError::NotFound { entity_type: "Sprint", entity_id: sprint_id.to_string() }
            }
            other => ToolError::from_core(other, request_id),
        })?;

    scope::check_project_access(state, token, &sprint.project_id, request_id).await?;

    // Ensure active link exists
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sprint_tasks WHERE sprint_id = ? AND task_id = ? AND removed_at IS NULL",
    )
    .bind(sprint_id)
    .bind(task_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ToolError::Internal { request_id: request_id.to_string() })?;

    if active == 0 {
        return Err(ToolError::NotFound {
            entity_type: "SprintTask",
            entity_id: format!("({sprint_id}, {task_id})"),
        });
    }

    SqliteSprintTaskRepository(state.pool.clone())
        .soft_remove(sprint_id, task_id)
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    Ok(json!({ "schema_version": 1, "removed": true }))
}

fn require_str<'a>(args: &'a Value, field: &'static str) -> Result<&'a str, ToolError> {
    args[field].as_str().ok_or_else(|| {
        let mut fields = HashMap::new();
        fields.insert(field.to_string(), "required".to_string());
        ToolError::ValidationError { fields }
    })
}

fn parse_date(args: &Value, field: &'static str) -> Result<NaiveDate, ToolError> {
    let s = args[field].as_str().ok_or_else(|| {
        let mut fields = HashMap::new();
        fields.insert(field.to_string(), "required".to_string());
        ToolError::ValidationError { fields }
    })?;
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
        let mut fields = HashMap::new();
        fields.insert(field.to_string(), "must be ISO-8601 date (YYYY-MM-DD)".to_string());
        ToolError::ValidationError { fields }
    })
}

fn parse_sprint_status(args: &Value) -> Result<Option<SprintStatus>, ToolError> {
    match args["status"].as_str() {
        None => Ok(None),
        Some(s) => {
            let status: Result<SprintStatus, _> =
                serde_json::from_value(Value::String(s.to_string()));
            status.map(Some).map_err(|_| ToolError::InvalidEnum {
                field: "status".to_string(),
                allowed: vec!["planned", "active", "closed"],
                received: s.to_string(),
            })
        }
    }
}
