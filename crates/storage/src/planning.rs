use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use roadboard_core::{
    planning::{
        Milestone, MilestoneRepository, MilestoneStatus, NewMilestone, NewSprint, NewSprintTask,
        NewTask, NewTaskDependency, Sprint, SprintRepository, SprintStatus, SprintTask,
        SprintTaskRepository, Task, TaskDependency, TaskDependencyRepository, TaskFilters,
        TaskRepository, TaskStatus, UpdateTask,
    },
    Error as CoreError, Result,
};

use crate::util::{
    enum_to_str, map_sqlx_err, parse_date, parse_date_opt, parse_enum, parse_ts, parse_ts_opt,
    req,
};

pub struct SqliteMilestoneRepository(pub SqlitePool);
pub struct SqliteSprintRepository(pub SqlitePool);
pub struct SqliteTaskRepository(pub SqlitePool);
pub struct SqliteTaskDependencyRepository(pub SqlitePool);
pub struct SqliteSprintTaskRepository(pub SqlitePool);

#[allow(clippy::too_many_arguments)]
fn row_to_milestone(
    id: Option<String>,
    project_id: String,
    title: String,
    description: Option<String>,
    due_date: Option<String>,
    status: String,
    order_index: i64,
    created_by_user_id: String,
    updated_by_user_id: String,
    created_at: String,
    updated_at: String,
) -> Result<Milestone> {
    Ok(Milestone {
        id: req(id, "milestone.id")?,
        project_id,
        title,
        description,
        due_date: parse_date_opt(due_date)?,
        status: parse_enum(&status)?,
        order_index,
        created_by_user_id,
        updated_by_user_id,
        created_at: parse_ts(&created_at)?,
        updated_at: parse_ts(&updated_at)?,
    })
}

#[allow(clippy::too_many_arguments)]
fn row_to_sprint(
    id: Option<String>,
    project_id: String,
    name: String,
    goal: Option<String>,
    start_date: String,
    end_date: String,
    status: String,
    created_by_user_id: String,
    updated_by_user_id: String,
    created_at: String,
    updated_at: String,
) -> Result<Sprint> {
    Ok(Sprint {
        id: req(id, "sprint.id")?,
        project_id,
        name,
        goal,
        start_date: parse_date(&start_date)?,
        end_date: parse_date(&end_date)?,
        status: parse_enum(&status)?,
        created_by_user_id,
        updated_by_user_id,
        created_at: parse_ts(&created_at)?,
        updated_at: parse_ts(&updated_at)?,
    })
}

#[allow(clippy::too_many_arguments)]
fn row_to_task(
    id: Option<String>,
    project_id: String,
    milestone_id: Option<String>,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    assignee_user_id: Option<String>,
    estimate: Option<String>,
    due_date: Option<String>,
    created_by_user_id: String,
    updated_by_user_id: String,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
) -> Result<Task> {
    Ok(Task {
        id: req(id, "task.id")?,
        project_id,
        milestone_id,
        title,
        description,
        status: parse_enum(&status)?,
        priority: parse_enum(&priority)?,
        assignee_user_id,
        estimate,
        due_date: parse_date_opt(due_date)?,
        created_by_user_id,
        updated_by_user_id,
        completed_at: parse_ts_opt(completed_at)?,
        created_at: parse_ts(&created_at)?,
        updated_at: parse_ts(&updated_at)?,
    })
}

impl MilestoneRepository for SqliteMilestoneRepository {
    async fn create(&self, new: NewMilestone) -> Result<Milestone> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let status_str = enum_to_str(&new.status)?;
        let due_date_str = new.due_date.map(|d| d.format("%Y-%m-%d").to_string());

        sqlx::query!(
            "INSERT INTO milestones (id, project_id, title, description, due_date, status,
             order_index, created_by_user_id, updated_by_user_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            id, new.project_id, new.title, new.description, due_date_str, status_str,
            new.order_index, new.created_by_user_id, new.updated_by_user_id, now, now
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find_by_id(&id).await
    }

    async fn find_by_id(&self, id: &str) -> Result<Milestone> {
        let r = sqlx::query!(
            "SELECT id, project_id, title, description, due_date, status, order_index,
                    created_by_user_id, updated_by_user_id, created_at, updated_at
             FROM milestones WHERE id = ?",
            id
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound { entity_type: "Milestone", id: id.to_string() })?;

        row_to_milestone(r.id, r.project_id, r.title, r.description, r.due_date, r.status,
                         r.order_index, r.created_by_user_id, r.updated_by_user_id,
                         r.created_at, r.updated_at)
    }

    async fn list_for_project(
        &self,
        project_id: &str,
        status: Option<MilestoneStatus>,
        after_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Milestone>> {
        let status_str = status.map(|s| enum_to_str(&s)).transpose()?;
        let status_ref = status_str.as_deref();

        let rows = sqlx::query!(
            "SELECT id, project_id, title, description, due_date, status, order_index,
                    created_by_user_id, updated_by_user_id, created_at, updated_at
             FROM milestones
             WHERE project_id = ?
               AND (? IS NULL OR status = ?)
               AND (? IS NULL OR id > ?)
             ORDER BY id LIMIT ?",
            project_id, status_ref, status_ref, after_id, after_id, limit
        )
        .fetch_all(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| row_to_milestone(r.id, r.project_id, r.title, r.description, r.due_date,
                                       r.status, r.order_index, r.created_by_user_id,
                                       r.updated_by_user_id, r.created_at, r.updated_at))
            .collect()
    }
}

impl SprintRepository for SqliteSprintRepository {
    async fn create(&self, new: NewSprint) -> Result<Sprint> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let status_str = enum_to_str(&new.status)?;
        let start_date_str = new.start_date.format("%Y-%m-%d").to_string();
        let end_date_str = new.end_date.format("%Y-%m-%d").to_string();

        sqlx::query!(
            "INSERT INTO sprints (id, project_id, name, goal, start_date, end_date, status,
             created_by_user_id, updated_by_user_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            id, new.project_id, new.name, new.goal, start_date_str, end_date_str, status_str,
            new.created_by_user_id, new.updated_by_user_id, now, now
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find_by_id(&id).await
    }

    async fn find_by_id(&self, id: &str) -> Result<Sprint> {
        let r = sqlx::query!(
            "SELECT id, project_id, name, goal, start_date, end_date, status,
                    created_by_user_id, updated_by_user_id, created_at, updated_at
             FROM sprints WHERE id = ?",
            id
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound { entity_type: "Sprint", id: id.to_string() })?;

        row_to_sprint(r.id, r.project_id, r.name, r.goal, r.start_date, r.end_date, r.status,
                      r.created_by_user_id, r.updated_by_user_id, r.created_at, r.updated_at)
    }

    async fn find_active_for_project(&self, project_id: &str) -> Result<Option<Sprint>> {
        let r = sqlx::query!(
            "SELECT id, project_id, name, goal, start_date, end_date, status,
                    created_by_user_id, updated_by_user_id, created_at, updated_at
             FROM sprints WHERE project_id = ? AND status = 'active' LIMIT 1",
            project_id
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        r.map(|r| {
            row_to_sprint(r.id, r.project_id, r.name, r.goal, r.start_date, r.end_date,
                          r.status, r.created_by_user_id, r.updated_by_user_id,
                          r.created_at, r.updated_at)
        })
        .transpose()
    }

    async fn list_for_project(
        &self,
        project_id: &str,
        status: Option<SprintStatus>,
        after_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Sprint>> {
        let status_str = status.map(|s| enum_to_str(&s)).transpose()?;
        let status_ref = status_str.as_deref();

        let rows = sqlx::query!(
            "SELECT id, project_id, name, goal, start_date, end_date, status,
                    created_by_user_id, updated_by_user_id, created_at, updated_at
             FROM sprints
             WHERE project_id = ?
               AND (? IS NULL OR status = ?)
               AND (? IS NULL OR id > ?)
             ORDER BY id LIMIT ?",
            project_id, status_ref, status_ref, after_id, after_id, limit
        )
        .fetch_all(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| row_to_sprint(r.id, r.project_id, r.name, r.goal, r.start_date, r.end_date,
                                    r.status, r.created_by_user_id, r.updated_by_user_id,
                                    r.created_at, r.updated_at))
            .collect()
    }
}

impl TaskRepository for SqliteTaskRepository {
    async fn create(&self, new: NewTask) -> Result<Task> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let status_str = enum_to_str(&new.status)?;
        let priority_str = enum_to_str(&new.priority)?;
        let due_date_str = new.due_date.map(|d| d.format("%Y-%m-%d").to_string());

        sqlx::query!(
            "INSERT INTO tasks (id, project_id, milestone_id, title, description, status, priority,
             assignee_user_id, estimate, due_date, created_by_user_id, updated_by_user_id,
             created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            id, new.project_id, new.milestone_id, new.title, new.description, status_str,
            priority_str, new.assignee_user_id, new.estimate, due_date_str,
            new.created_by_user_id, new.updated_by_user_id, now, now
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find_by_id(&id).await
    }

    async fn find_by_id(&self, id: &str) -> Result<Task> {
        let r = sqlx::query!(
            "SELECT id, project_id, milestone_id, title, description, status, priority,
                    assignee_user_id, estimate, due_date, created_by_user_id, updated_by_user_id,
                    completed_at, created_at, updated_at
             FROM tasks WHERE id = ?",
            id
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound { entity_type: "Task", id: id.to_string() })?;

        row_to_task(r.id, r.project_id, r.milestone_id, r.title, r.description, r.status,
                    r.priority, r.assignee_user_id, r.estimate, r.due_date,
                    r.created_by_user_id, r.updated_by_user_id, r.completed_at,
                    r.created_at, r.updated_at)
    }

    async fn list_for_project(
        &self,
        project_id: &str,
        filters: TaskFilters,
        after_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Task>> {
        let status_str = filters.status.map(|s| enum_to_str(&s)).transpose()?;
        let status_ref = status_str.as_deref();
        let milestone_ref = filters.milestone_id.as_deref();
        let assignee_ref = filters.assignee_user_id.as_deref();
        let sprint_ref = filters.sprint_id.as_deref();

        let rows = sqlx::query!(
            "SELECT id, project_id, milestone_id, title, description, status, priority,
                    assignee_user_id, estimate, due_date, created_by_user_id, updated_by_user_id,
                    completed_at, created_at, updated_at
             FROM tasks
             WHERE project_id = ?
               AND (? IS NULL OR status = ?)
               AND (? IS NULL OR milestone_id = ?)
               AND (? IS NULL OR assignee_user_id = ?)
               AND (? IS NULL OR EXISTS (
                   SELECT 1 FROM sprint_tasks st
                   WHERE st.task_id = tasks.id AND st.sprint_id = ? AND st.removed_at IS NULL
               ))
               AND (? IS NULL OR id > ?)
             ORDER BY id LIMIT ?",
            project_id,
            status_ref, status_ref,
            milestone_ref, milestone_ref,
            assignee_ref, assignee_ref,
            sprint_ref, sprint_ref,
            after_id, after_id,
            limit
        )
        .fetch_all(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| row_to_task(r.id, r.project_id, r.milestone_id, r.title, r.description,
                                   r.status, r.priority, r.assignee_user_id, r.estimate,
                                   r.due_date, r.created_by_user_id, r.updated_by_user_id,
                                   r.completed_at, r.created_at, r.updated_at))
            .collect()
    }

    async fn update(&self, id: &str, patch: UpdateTask) -> Result<Task> {
        let now = Utc::now().to_rfc3339();
        let priority_str = patch.priority.as_ref().map(enum_to_str).transpose()?;

        sqlx::query!(
            "UPDATE tasks SET
               title            = COALESCE(?, title),
               description      = COALESCE(?, description),
               milestone_id     = COALESCE(?, milestone_id),
               priority         = COALESCE(?, priority),
               assignee_user_id = COALESCE(?, assignee_user_id),
               due_date         = COALESCE(?, due_date),
               estimate         = COALESCE(?, estimate),
               updated_at       = ?
             WHERE id = ?",
            patch.title, patch.description, patch.milestone_id, priority_str,
            patch.assignee_user_id, patch.due_date, patch.estimate, now, id
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find_by_id(id).await
    }

    async fn update_status(&self, id: &str, status: TaskStatus) -> Result<Task> {
        let now = Utc::now().to_rfc3339();
        let status_str = enum_to_str(&status)?;
        let completed_at: Option<&str> = if status == TaskStatus::Done { Some(&now) } else { None };

        sqlx::query!(
            "UPDATE tasks SET status = ?, completed_at = ?, updated_at = ? WHERE id = ?",
            status_str, completed_at, now, id
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find_by_id(id).await
    }
}

impl TaskDependencyRepository for SqliteTaskDependencyRepository {
    async fn add(&self, new: NewTaskDependency) -> Result<TaskDependency> {
        if new.from_task_id == new.to_task_id {
            return Err(CoreError::InvalidInput(
                "a task cannot depend on itself".to_string(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let dep_type_str = enum_to_str(&new.dependency_type)?;

        sqlx::query!(
            "INSERT INTO task_dependencies (from_task_id, to_task_id, dependency_type, created_at)
             VALUES (?, ?, ?, ?)",
            new.from_task_id, new.to_task_id, dep_type_str, now
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        let r = sqlx::query!(
            "SELECT from_task_id, to_task_id, dependency_type, created_at
             FROM task_dependencies WHERE from_task_id = ? AND to_task_id = ?",
            new.from_task_id, new.to_task_id
        )
        .fetch_one(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        Ok(TaskDependency {
            from_task_id: r.from_task_id,
            to_task_id: r.to_task_id,
            dependency_type: parse_enum(&r.dependency_type)?,
            created_at: parse_ts(&r.created_at)?,
        })
    }

    async fn remove(&self, from_id: &str, to_id: &str) -> Result<()> {
        sqlx::query!(
            "DELETE FROM task_dependencies WHERE from_task_id = ? AND to_task_id = ?",
            from_id, to_id
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn list_for_task(&self, task_id: &str) -> Result<Vec<TaskDependency>> {
        let rows = sqlx::query!(
            "SELECT from_task_id, to_task_id, dependency_type, created_at
             FROM task_dependencies WHERE from_task_id = ? OR to_task_id = ?
             ORDER BY created_at",
            task_id, task_id
        )
        .fetch_all(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| {
                Ok(TaskDependency {
                    from_task_id: r.from_task_id,
                    to_task_id: r.to_task_id,
                    dependency_type: parse_enum(&r.dependency_type)?,
                    created_at: parse_ts(&r.created_at)?,
                })
            })
            .collect()
    }
}

impl SprintTaskRepository for SqliteSprintTaskRepository {
    async fn add(&self, new: NewSprintTask) -> Result<SprintTask> {
        let now = Utc::now().to_rfc3339();

        sqlx::query!(
            "INSERT INTO sprint_tasks (sprint_id, task_id, added_at, carried_from_sprint_id, removed_at)
             VALUES (?, ?, ?, ?, NULL)
             ON CONFLICT(sprint_id, task_id) DO UPDATE SET
               added_at               = excluded.added_at,
               carried_from_sprint_id = excluded.carried_from_sprint_id,
               removed_at             = NULL",
            new.sprint_id, new.task_id, now, new.carried_from_sprint_id
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        let r = sqlx::query!(
            "SELECT sprint_id, task_id, added_at, carried_from_sprint_id, removed_at
             FROM sprint_tasks WHERE sprint_id = ? AND task_id = ?",
            new.sprint_id, new.task_id
        )
        .fetch_one(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        Ok(SprintTask {
            sprint_id: r.sprint_id,
            task_id: r.task_id,
            added_at: parse_ts(&r.added_at)?,
            carried_from_sprint_id: r.carried_from_sprint_id,
            removed_at: parse_ts_opt(r.removed_at)?,
        })
    }

    async fn soft_remove(&self, sprint_id: &str, task_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query!(
            "UPDATE sprint_tasks SET removed_at = ? WHERE sprint_id = ? AND task_id = ?",
            now, sprint_id, task_id
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn list_for_sprint(&self, sprint_id: &str) -> Result<Vec<SprintTask>> {
        let rows = sqlx::query!(
            "SELECT sprint_id, task_id, added_at, carried_from_sprint_id, removed_at
             FROM sprint_tasks WHERE sprint_id = ? ORDER BY added_at",
            sprint_id
        )
        .fetch_all(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| {
                Ok(SprintTask {
                    sprint_id: r.sprint_id,
                    task_id: r.task_id,
                    added_at: parse_ts(&r.added_at)?,
                    carried_from_sprint_id: r.carried_from_sprint_id,
                    removed_at: parse_ts_opt(r.removed_at)?,
                })
            })
            .collect()
    }

    async fn list_for_task(&self, task_id: &str) -> Result<Vec<SprintTask>> {
        let rows = sqlx::query!(
            "SELECT sprint_id, task_id, added_at, carried_from_sprint_id, removed_at
             FROM sprint_tasks WHERE task_id = ? ORDER BY added_at",
            task_id
        )
        .fetch_all(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| {
                Ok(SprintTask {
                    sprint_id: r.sprint_id,
                    task_id: r.task_id,
                    added_at: parse_ts(&r.added_at)?,
                    carried_from_sprint_id: r.carried_from_sprint_id,
                    removed_at: parse_ts_opt(r.removed_at)?,
                })
            })
            .collect()
    }
}
