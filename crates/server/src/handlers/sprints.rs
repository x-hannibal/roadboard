use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::NaiveDate;
use roadboard_core::planning::{
    NewSprint, NewSprintTask, SprintRepository, SprintStatus, SprintTaskRepository,
};
use roadboard_storage::{SqliteSprintRepository, SqliteSprintTaskRepository};
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

use crate::error::AppError;
use crate::state::AppState;
use super::{decode_cursor, paginate, parse_limit, require_project_access, session_user_id};

#[derive(Deserialize)]
pub struct SprintListQuery {
    pub status: Option<SprintStatus>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_sprints(
    session: Session,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<SprintListQuery>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    require_project_access(&state, &user_id, &project_id).await?;
    let limit = parse_limit(q.limit);
    let after = decode_cursor(q.cursor.as_deref())?;

    let sprints = SqliteSprintRepository(state.pool.clone())
        .list_for_project(&project_id, q.status, after.as_deref(), limit)
        .await
        .map_err(AppError::from)?;

    let page = paginate(&sprints, limit, |s| s.id.as_str());
    Ok(Json(page).into_response())
}

#[derive(Deserialize)]
pub struct CreateSprintBody {
    pub name: String,
    pub goal: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: Option<SprintStatus>,
}

pub async fn create_sprint(
    session: Session,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateSprintBody>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    require_project_access(&state, &user_id, &project_id).await?;

    let sprint = SqliteSprintRepository(state.pool.clone())
        .create(NewSprint {
            project_id,
            name: body.name,
            goal: body.goal,
            start_date: body.start_date,
            end_date: body.end_date,
            status: body.status.unwrap_or(SprintStatus::Planned),
            created_by_user_id: user_id.clone(),
            updated_by_user_id: user_id,
        })
        .await
        .map_err(AppError::from)?;

    Ok((StatusCode::CREATED, Json(json!({ "sprint": sprint }))).into_response())
}

pub async fn get_sprint(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let sprint = SqliteSprintRepository(state.pool.clone())
        .find_by_id(&id)
        .await
        .map_err(AppError::from)?;
    require_project_access(&state, &user_id, &sprint.project_id).await?;
    Ok(Json(json!({ "sprint": sprint })).into_response())
}

pub async fn get_active_sprint(
    session: Session,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    require_project_access(&state, &user_id, &project_id).await?;
    let sprint = SqliteSprintRepository(state.pool.clone())
        .find_active_for_project(&project_id)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "sprint": sprint })).into_response())
}

#[derive(Deserialize)]
pub struct AddSprintTaskBody {
    pub task_id: String,
    pub carried_from_sprint_id: Option<String>,
}

pub async fn add_task_to_sprint(
    session: Session,
    State(state): State<AppState>,
    Path(sprint_id): Path<String>,
    Json(body): Json<AddSprintTaskBody>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let sprint = SqliteSprintRepository(state.pool.clone())
        .find_by_id(&sprint_id)
        .await
        .map_err(AppError::from)?;
    require_project_access(&state, &user_id, &sprint.project_id).await?;

    let st = SqliteSprintTaskRepository(state.pool.clone())
        .add(NewSprintTask {
            sprint_id,
            task_id: body.task_id,
            carried_from_sprint_id: body.carried_from_sprint_id,
        })
        .await
        .map_err(AppError::from)?;

    Ok((StatusCode::CREATED, Json(json!({ "sprint_task": st }))).into_response())
}

pub async fn remove_task_from_sprint(
    session: Session,
    State(state): State<AppState>,
    Path((sprint_id, task_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let user_id = session_user_id(&session).await?;
    let sprint = SqliteSprintRepository(state.pool.clone())
        .find_by_id(&sprint_id)
        .await
        .map_err(AppError::from)?;
    require_project_access(&state, &user_id, &sprint.project_id).await?;

    SqliteSprintTaskRepository(state.pool.clone())
        .soft_remove(&sprint_id, &task_id)
        .await
        .map_err(AppError::from)?;

    Ok(StatusCode::NO_CONTENT)
}
