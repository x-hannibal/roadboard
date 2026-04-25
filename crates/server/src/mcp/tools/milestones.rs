use chrono::NaiveDate;
use roadboard_core::{
    planning::{MilestoneRepository, MilestoneStatus, NewMilestone},
    user::{GrantType, McpToken},
};
use roadboard_storage::SqliteMilestoneRepository;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::state::AppState;
use super::super::error::ToolError;
use super::{pagination, scope};

pub async fn list_milestones(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectRead)?;

    let project_id = require_str(&args, "projectId")?;
    scope::check_project_access(state, token, project_id, request_id).await?;

    let status = parse_milestone_status(&args)?;
    let limit = pagination::parse_limit(&args)?;
    let after_id = pagination::parse_cursor(&args)?;

    let milestones = SqliteMilestoneRepository(state.pool.clone())
        .list_for_project(project_id, status, after_id.as_deref(), limit)
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    let items: Vec<Value> =
        milestones.iter().map(|m| serde_json::to_value(m).unwrap_or(Value::Null)).collect();

    Ok(pagination::build_page(items, limit))
}

pub async fn create_milestone(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectWrite)?;

    let project_id = require_str(&args, "projectId")?;
    scope::check_project_access(state, token, project_id, request_id).await?;

    let title = require_str(&args, "title")?;

    let due_date = match args["dueDate"].as_str() {
        None => None,
        Some(s) => {
            let d = NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
                let mut fields = HashMap::new();
                fields.insert("dueDate".to_string(), "must be ISO-8601 date (YYYY-MM-DD)".to_string());
                ToolError::ValidationError { fields }
            })?;
            Some(d)
        }
    };

    let description = args["description"].as_str().map(str::to_string);
    let order_index = args["orderIndex"]
        .as_i64()
        .or_else(|| args["orderIndex"].as_f64().map(|f| f as i64))
        .unwrap_or(0);

    let milestone = SqliteMilestoneRepository(state.pool.clone())
        .create(NewMilestone {
            project_id: project_id.to_string(),
            title: title.to_string(),
            description,
            due_date,
            status: MilestoneStatus::Planned,
            order_index,
            created_by_user_id: token.user_id.clone(),
            updated_by_user_id: token.user_id.clone(),
        })
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    Ok(json!({ "schema_version": 1, "milestone": milestone }))
}

fn require_str<'a>(args: &'a Value, field: &'static str) -> Result<&'a str, ToolError> {
    args[field].as_str().ok_or_else(|| {
        let mut fields = HashMap::new();
        fields.insert(field.to_string(), "required".to_string());
        ToolError::ValidationError { fields }
    })
}

fn parse_milestone_status(args: &Value) -> Result<Option<MilestoneStatus>, ToolError> {
    match args["status"].as_str() {
        None => Ok(None),
        Some(s) => {
            let status: Result<MilestoneStatus, _> =
                serde_json::from_value(Value::String(s.to_string()));
            status.map(Some).map_err(|_| ToolError::InvalidEnum {
                field: "status".to_string(),
                allowed: vec!["planned", "in_progress", "done", "cancelled"],
                received: s.to_string(),
            })
        }
    }
}
