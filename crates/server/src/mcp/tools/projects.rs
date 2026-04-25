use roadboard_core::{
    project::{NewProject, NewProjectMember, ProjectMemberRepository, ProjectRepository, ProjectRole},
    user::{GrantType, McpToken},
};
use roadboard_storage::{SqliteProjectMemberRepository, SqliteProjectRepository};
use serde_json::{json, Value};

use crate::state::AppState;
use super::super::error::ToolError;
use super::{pagination, scope};

pub async fn list_projects(
    args: Value,
    token: &McpToken,
    state: &AppState,
    _request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectRead)?;
    let limit = pagination::parse_limit(&args)?;
    let after_id = pagination::parse_cursor(&args)?;

    let projects = SqliteProjectRepository(state.pool.clone())
        .list_accessible_to_user(&token.user_id, after_id.as_deref(), limit)
        .await
        .map_err(|e| ToolError::from_core(e, _request_id))?;

    let items: Vec<Value> = projects
        .iter()
        .map(|p| serde_json::to_value(p).unwrap_or(Value::Null))
        .collect();

    Ok(pagination::build_page(items, limit))
}

pub async fn get_project(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectRead)?;

    let project_id = args["projectId"].as_str().ok_or_else(|| {
        let mut fields = std::collections::HashMap::new();
        fields.insert("projectId".to_string(), "required".to_string());
        ToolError::ValidationError { fields }
    })?;

    scope::check_project_access(state, token, project_id, request_id).await?;

    let project = SqliteProjectRepository(state.pool.clone())
        .find_by_id(project_id)
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    let milestone_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM milestones WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ToolError::Internal { request_id: request_id.to_string() })?;

    let sprint_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sprints WHERE project_id = ?")
            .bind(project_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ToolError::Internal { request_id: request_id.to_string() })?;

    let task_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE project_id = ?")
            .bind(project_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ToolError::Internal { request_id: request_id.to_string() })?;

    let member_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_members WHERE project_id = ?")
            .bind(project_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| ToolError::Internal { request_id: request_id.to_string() })?;

    Ok(json!({
        "schema_version": 1,
        "project": project,
        "counts": {
            "milestones": milestone_count,
            "sprints": sprint_count,
            "tasks": task_count,
            "members": member_count
        }
    }))
}

pub async fn create_project(
    args: Value,
    token: &McpToken,
    state: &AppState,
    request_id: &str,
) -> Result<Value, ToolError> {
    scope::check_scope(token, GrantType::ProjectAdmin)?;

    let name = args["name"].as_str().ok_or_else(|| {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), "required".to_string());
        ToolError::ValidationError { fields }
    })?;

    let slug = match args["slug"].as_str() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => slugify(name),
    };
    let description = args["description"].as_str().map(str::to_string);

    let project = SqliteProjectRepository(state.pool.clone())
        .create(NewProject {
            slug,
            name: name.to_string(),
            description,
            owner_user_id: token.user_id.clone(),
        })
        .await
        .map_err(|e| match e {
            roadboard_core::Error::Conflict(msg) => {
                ToolError::Conflict { constraint: msg, existing_id: None }
            }
            other => ToolError::from_core(other, request_id),
        })?;

    // Auto-create Owner member
    SqliteProjectMemberRepository(state.pool.clone())
        .add(NewProjectMember {
            project_id: project.id.clone(),
            user_id: token.user_id.clone(),
            role: ProjectRole::Owner,
            granted_by_user_id: token.user_id.clone(),
        })
        .await
        .map_err(|e| ToolError::from_core(e, request_id))?;

    Ok(json!({ "schema_version": 1, "project": project }))
}

fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else {
            slug.push('-');
        }
    }
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() { "project".to_string() } else { slug }
}
