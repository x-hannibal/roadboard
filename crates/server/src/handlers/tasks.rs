use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::NaiveDate;
use roadboard_core::planning::{
    NewTask, TaskFilters, TaskPriority, TaskRepository, TaskStatus, UpdateTask,
};
use roadboard_storage::SqliteTaskRepository;
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

use crate::error::AppError;
use crate::state::AppState;
use super::{decode_cursor, paginate, parse_limit, require_project_access, session_user_id};

#[derive(Deserialize, Default)]
pub struct TaskListQuery {
    pub status: Option<TaskStatus>,
    pub milestone_id: Option<String>,
    pub sprint_id: Option<String>,
    pub assignee_user_id: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_tasks(
    session: Session,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<TaskListQuery>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    require_project_access(&state, &user_id, &project_id).await?;
    let limit = parse_limit(q.limit);
    let after = decode_cursor(q.cursor.as_deref())?;

    let tasks = SqliteTaskRepository(state.pool.clone())
        .list_for_project(
            &project_id,
            TaskFilters {
                status: q.status,
                milestone_id: q.milestone_id,
                sprint_id: q.sprint_id,
                assignee_user_id: q.assignee_user_id,
            },
            after.as_deref(),
            limit,
        )
        .await
        .map_err(AppError::from)?;

    let page = paginate(&tasks, limit, |t| t.id.as_str());
    Ok(Json(page).into_response())
}

#[derive(Deserialize)]
pub struct CreateTaskBody {
    pub title: String,
    pub description: Option<String>,
    pub milestone_id: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub assignee_user_id: Option<String>,
    pub estimate: Option<String>,
    pub due_date: Option<NaiveDate>,
}

pub async fn create_task(
    session: Session,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateTaskBody>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    require_project_access(&state, &user_id, &project_id).await?;

    let task = SqliteTaskRepository(state.pool.clone())
        .create(NewTask {
            project_id,
            milestone_id: body.milestone_id,
            title: body.title,
            description: body.description,
            status: body.status.unwrap_or(TaskStatus::Todo),
            priority: body.priority.unwrap_or(TaskPriority::Medium),
            assignee_user_id: body.assignee_user_id,
            estimate: body.estimate,
            due_date: body.due_date,
            created_by_user_id: user_id.clone(),
            updated_by_user_id: user_id,
        })
        .await
        .map_err(AppError::from)?;

    Ok((StatusCode::CREATED, Json(json!({ "task": task }))).into_response())
}

pub async fn get_task(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let task = SqliteTaskRepository(state.pool.clone())
        .find_by_id(&id)
        .await
        .map_err(AppError::from)?;
    require_project_access(&state, &user_id, &task.project_id).await?;
    Ok(Json(json!({ "task": task })).into_response())
}

#[derive(Deserialize)]
pub struct UpdateTaskBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub milestone_id: Option<String>,
    pub priority: Option<TaskPriority>,
    pub assignee_user_id: Option<String>,
    pub due_date: Option<String>,
    pub estimate: Option<String>,
}

pub async fn update_task(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTaskBody>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let existing = SqliteTaskRepository(state.pool.clone())
        .find_by_id(&id)
        .await
        .map_err(AppError::from)?;
    require_project_access(&state, &user_id, &existing.project_id).await?;

    let task = SqliteTaskRepository(state.pool.clone())
        .update(
            &id,
            UpdateTask {
                title: body.title,
                description: body.description,
                milestone_id: body.milestone_id,
                priority: body.priority,
                assignee_user_id: body.assignee_user_id,
                due_date: body.due_date,
                estimate: body.estimate,
            },
        )
        .await
        .map_err(AppError::from)?;

    Ok(Json(json!({ "task": task })).into_response())
}

#[derive(Deserialize)]
pub struct UpdateStatusBody {
    pub status: TaskStatus,
}

pub async fn update_task_status(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateStatusBody>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let existing = SqliteTaskRepository(state.pool.clone())
        .find_by_id(&id)
        .await
        .map_err(AppError::from)?;
    require_project_access(&state, &user_id, &existing.project_id).await?;

    let task = SqliteTaskRepository(state.pool.clone())
        .update_status(&id, body.status)
        .await
        .map_err(AppError::from)?;

    Ok(Json(json!({ "task": task })).into_response())
}
