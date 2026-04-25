use chrono::NaiveDate;
use roadboard_core::{
    planning::{
        MilestoneRepository, NewSprintTask, NewTask, SprintRepository, SprintTaskRepository,
        TaskFilters, TaskPriority, TaskRepository, TaskStatus, UpdateTask,
    },
    user::{GrantType, McpToken},
};
use roadboard_storage::{
    SqliteMilestoneRepository, SqliteSprintRepository, SqliteSprintTaskRepository,
    SqliteTaskRepository,
};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::state::AppState;
use super::super::error::ToolError;
use super::{pagination, scope};

pub async fn list_tasks(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectRead)?;
    let project_id = require_str(&args, "projectId")?;
    scope::check_project_access(state, token, project_id, request_id).await?;

    let status = parse_task_status_opt(&args)?;
    let milestone_id = args["milestoneId"].as_str().map(str::to_string);
    let sprint_id = args["sprintId"].as_str().map(str::to_string);
    let assignee_user_id = args["assigneeUserId"].as_str().map(str::to_string);
    let limit = pagination::parse_limit(&args)?;
    let after_id = pagination::parse_cursor(&args)?;

    let tasks = SqliteTaskRepository(state.pool.clone())
        .list_for_project(
            project_id,
            TaskFilters { status, milestone_id, sprint_id, assignee_user_id },
            after_id.as_deref(),
            limit,
        )
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    let items: Vec<Value> =
        tasks.iter().map(|t| serde_json::to_value(t).unwrap_or(Value::Null)).collect();

    Ok(pagination::build_page(items, limit))
}

pub async fn get_task(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectRead)?;
    let task_id = require_str(&args, "taskId")?;

    let task = SqliteTaskRepository(state.pool.clone())
        .find_by_id(task_id)
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    scope::check_project_access(state, token, &task.project_id, request_id).await?;

    Ok(json!({ "schema_version": 1, "task": task }))
}

pub async fn create_task(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::TaskWrite)?;
    let project_id = require_str(&args, "projectId")?;
    scope::check_project_access(state, token, project_id, request_id).await?;

    let title = require_str(&args, "title")?;
    let description = args["description"].as_str().map(str::to_string);
    let assignee_user_id = args["assigneeUserId"].as_str().map(str::to_string);
    let estimate = args["estimate"].as_str().map(str::to_string);
    let due_date = parse_date_opt(&args, "dueDate")?;

    let priority = match args["priority"].as_str() {
        None => TaskPriority::Medium,
        Some(s) => serde_json::from_value(Value::String(s.to_string())).map_err(|_| {
            ToolError::InvalidEnum {
                field: "priority".to_string(),
                allowed: vec!["low", "medium", "high", "urgent"],
                received: s.to_string(),
            }
        })?,
    };

    // Optional milestone validation
    let milestone_id = match args["milestoneId"].as_str() {
        None => None,
        Some(mid) => {
            let ms = SqliteMilestoneRepository(state.pool.clone())
                .find_by_id(mid)
                .await
                .map_err(|e| match e {
                    roadboard_core::Error::NotFound { .. } => {
                        ToolError::InvalidReference { field: "milestoneId", value: mid.to_string() }
                    }
                    other => ToolError::from_core(other, request_id),
                })?;
            if ms.project_id != project_id {
                return Err(ToolError::InvalidReference {
                    field: "milestoneId",
                    value: mid.to_string(),
                });
            }
            Some(mid.to_string())
        }
    };

    // Optional sprint validation
    let sprint_id_str = args["sprintId"].as_str().map(str::to_string);
    if let Some(ref sid) = sprint_id_str {
        let sp = SqliteSprintRepository(state.pool.clone())
            .find_by_id(sid)
            .await
            .map_err(|e| match e {
                roadboard_core::Error::NotFound { .. } => {
                    ToolError::NotFound { entity_type: "Sprint", entity_id: sid.clone() }
                }
                other => ToolError::from_core(other, request_id),
            })?;
        if sp.project_id != project_id {
            return Err(ToolError::TaskProjectMismatch);
        }
    }

    let task = SqliteTaskRepository(state.pool.clone())
        .create(NewTask {
            project_id: project_id.to_string(),
            milestone_id,
            title: title.to_string(),
            description,
            status: TaskStatus::Todo,
            priority,
            assignee_user_id,
            estimate,
            due_date,
            created_by_user_id: token.user_id.clone(),
            updated_by_user_id: token.user_id.clone(),
        })
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    // Link to sprint if provided
    if let Some(sid) = sprint_id_str {
        SqliteSprintTaskRepository(state.pool.clone())
            .add(NewSprintTask {
                sprint_id: sid,
                task_id: task.id.clone(),
                carried_from_sprint_id: None,
            })
            .await
            .map_err(|e| ToolError::from_core(e, request_id))?;
    }

    Ok(json!({ "schema_version": 1, "task": task }))
}

pub async fn update_task(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::TaskWrite)?;
    let task_id = require_str(&args, "taskId")?;

    // Ensure at least one field is present
    let title = args["title"].as_str().map(str::to_string);
    let description = args["description"].as_str().map(str::to_string);
    let milestone_id = args["milestoneId"].as_str().map(str::to_string);
    let assignee_user_id = args["assigneeUserId"].as_str().map(str::to_string);
    let due_date = args["dueDate"].as_str().map(str::to_string);
    let estimate = args["estimate"].as_str().map(str::to_string);
    let priority_str = args["priority"].as_str();

    if title.is_none()
        && description.is_none()
        && milestone_id.is_none()
        && assignee_user_id.is_none()
        && due_date.is_none()
        && estimate.is_none()
        && priority_str.is_none()
    {
        let mut fields = HashMap::new();
        fields.insert("_".to_string(), "at least one field required".to_string());
        return Err(ToolError::ValidationError { fields });
    }

    let priority = match priority_str {
        None => None,
        Some(s) => {
            let p: Result<TaskPriority, _> = serde_json::from_value(Value::String(s.to_string()));
            Some(p.map_err(|_| ToolError::InvalidEnum {
                field: "priority".to_string(),
                allowed: vec!["low", "medium", "high", "urgent"],
                received: s.to_string(),
            })?)
        }
    };

    let existing = SqliteTaskRepository(state.pool.clone())
        .find_by_id(task_id)
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    scope::check_project_access(state, token, &existing.project_id, request_id).await?;

    let task = SqliteTaskRepository(state.pool.clone())
        .update(task_id, UpdateTask { title, description, milestone_id, priority, assignee_user_id, due_date, estimate })
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    Ok(json!({ "schema_version": 1, "task": task }))
}

pub async fn update_task_status(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::TaskWrite)?;
    let task_id = require_str(&args, "taskId")?;

    let status_str = args["status"].as_str().ok_or_else(|| {
        let mut fields = HashMap::new();
        fields.insert("status".to_string(), "required".to_string());
        ToolError::ValidationError { fields }
    })?;

    let status: TaskStatus = serde_json::from_value(Value::String(status_str.to_string()))
        .map_err(|_| ToolError::InvalidEnum {
            field: "status".to_string(),
            allowed: vec!["todo", "in_progress", "blocked", "done", "cancelled"],
            received: status_str.to_string(),
        })?;

    let existing = SqliteTaskRepository(state.pool.clone())
        .find_by_id(task_id)
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    scope::check_project_access(state, token, &existing.project_id, request_id).await?;

    if let Some(report) = args["completionReport"].as_str() {
        // TODO(M2): persist completionReport as MemoryEntry when memory tables exist
        tracing::debug!(task_id, report, "completionReport ignored in M1");
    }

    let task = SqliteTaskRepository(state.pool.clone())
        .update_status(task_id, status)
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    Ok(json!({ "schema_version": 1, "task": task }))
}

fn require_str<'a>(args: &'a Value, field: &'static str) -> Result<&'a str, ToolError> {
    args[field].as_str().ok_or_else(|| {
        let mut fields = HashMap::new();
        fields.insert(field.to_string(), "required".to_string());
        ToolError::ValidationError { fields }
    })
}

fn parse_date_opt(args: &Value, field: &'static str) -> Result<Option<NaiveDate>, ToolError> {
    match args[field].as_str() {
        None => Ok(None),
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| {
                let mut fields = HashMap::new();
                fields.insert(field.to_string(), "must be ISO-8601 date (YYYY-MM-DD)".to_string());
                ToolError::ValidationError { fields }
            }),
    }
}

fn parse_task_status_opt(args: &Value) -> Result<Option<TaskStatus>, ToolError> {
    match args["status"].as_str() {
        None => Ok(None),
        Some(s) => {
            let status: Result<TaskStatus, _> =
                serde_json::from_value(Value::String(s.to_string()));
            status.map(Some).map_err(|_| ToolError::InvalidEnum {
                field: "status".to_string(),
                allowed: vec!["todo", "in_progress", "blocked", "done", "cancelled"],
                received: s.to_string(),
            })
        }
    }
}
