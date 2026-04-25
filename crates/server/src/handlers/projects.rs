use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use roadboard_core::project::{
    NewProject, NewProjectMember, ProjectMemberRepository, ProjectRepository, ProjectRole,
};
use roadboard_storage::{SqliteProjectMemberRepository, SqliteProjectRepository};
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

use crate::error::AppError;
use crate::state::AppState;
use super::{decode_cursor, paginate, parse_limit, require_project_access, session_user_id, PaginationQuery};

pub async fn list_projects(
    session: Session,
    State(state): State<AppState>,
    Query(q): Query<PaginationQuery>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let limit = parse_limit(q.limit);
    let after = decode_cursor(q.cursor.as_deref())?;

    let projects = SqliteProjectRepository(state.pool.clone())
        .list_accessible_to_user(&user_id, after.as_deref(), limit)
        .await
        .map_err(AppError::from)?;

    let page = paginate(&projects, limit, |p| p.id.as_str());
    Ok(Json(page).into_response())
}

#[derive(Deserialize)]
pub struct CreateProjectBody {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

pub async fn create_project(
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let project = SqliteProjectRepository(state.pool.clone())
        .create(NewProject {
            slug: body.slug,
            name: body.name,
            description: body.description,
            owner_user_id: user_id.clone(),
        })
        .await
        .map_err(AppError::from)?;

    // Auto-add creator as Owner
    SqliteProjectMemberRepository(state.pool.clone())
        .add(NewProjectMember {
            project_id: project.id.clone(),
            user_id: user_id.clone(),
            role: ProjectRole::Owner,
            granted_by_user_id: user_id,
        })
        .await
        .map_err(AppError::from)?;

    Ok((StatusCode::CREATED, Json(json!({ "project": project }))).into_response())
}

pub async fn get_project(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    require_project_access(&state, &user_id, &id).await?;
    let project = SqliteProjectRepository(state.pool.clone())
        .find_by_id(&id)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "project": project })).into_response())
}

pub async fn list_members(
    session: Session,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    require_project_access(&state, &user_id, &project_id).await?;
    let members = SqliteProjectMemberRepository(state.pool.clone())
        .list_for_project(&project_id)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "members": members })).into_response())
}

#[derive(Deserialize)]
pub struct AddMemberBody {
    pub user_id: String,
    pub role: ProjectRole,
}

pub async fn add_member(
    session: Session,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<AddMemberBody>,
) -> Result<Response, AppError> {
    let caller_id = session_user_id(&session).await?;
    require_project_access(&state, &caller_id, &project_id).await?;
    let member = SqliteProjectMemberRepository(state.pool.clone())
        .add(NewProjectMember {
            project_id,
            user_id: body.user_id,
            role: body.role,
            granted_by_user_id: caller_id,
        })
        .await
        .map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(json!({ "member": member }))).into_response())
}

pub async fn remove_member(
    session: Session,
    State(state): State<AppState>,
    Path((project_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let caller_id = session_user_id(&session).await?;
    require_project_access(&state, &caller_id, &project_id).await?;
    SqliteProjectMemberRepository(state.pool.clone())
        .remove(&project_id, &user_id)
        .await
        .map_err(AppError::from)?;
    Ok(StatusCode::NO_CONTENT)
}
