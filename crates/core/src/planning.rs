use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneStatus {
    Planned,
    InProgress,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SprintStatus {
    Planned,
    Active,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Blocks,
    Relates,
    Duplicates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub status: MilestoneStatus,
    pub order_index: i64,
    pub created_by_user_id: String,
    pub updated_by_user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewMilestone {
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub status: MilestoneStatus,
    pub order_index: i64,
    pub created_by_user_id: String,
    pub updated_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprint {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub goal: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: SprintStatus,
    pub created_by_user_id: String,
    pub updated_by_user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewSprint {
    pub project_id: String,
    pub name: String,
    pub goal: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: SprintStatus,
    pub created_by_user_id: String,
    pub updated_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub milestone_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assignee_user_id: Option<String>,
    pub estimate: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub created_by_user_id: String,
    pub updated_by_user_id: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewTask {
    pub project_id: String,
    pub milestone_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assignee_user_id: Option<String>,
    pub estimate: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub created_by_user_id: String,
    pub updated_by_user_id: String,
}

pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub milestone_id: Option<String>,
    pub priority: Option<TaskPriority>,
    pub assignee_user_id: Option<String>,
    pub due_date: Option<String>,
    pub estimate: Option<String>,
}

pub struct TaskFilters {
    pub status: Option<TaskStatus>,
    pub milestone_id: Option<String>,
    pub sprint_id: Option<String>,
    pub assignee_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub from_task_id: String,
    pub to_task_id: String,
    pub dependency_type: DependencyType,
    pub created_at: DateTime<Utc>,
}

pub struct NewTaskDependency {
    pub from_task_id: String,
    pub to_task_id: String,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintTask {
    pub sprint_id: String,
    pub task_id: String,
    pub added_at: DateTime<Utc>,
    pub carried_from_sprint_id: Option<String>,
    pub removed_at: Option<DateTime<Utc>>,
}

pub struct NewSprintTask {
    pub sprint_id: String,
    pub task_id: String,
    pub carried_from_sprint_id: Option<String>,
}

pub trait MilestoneRepository {
    async fn create(&self, new: NewMilestone) -> Result<Milestone>;
    async fn find_by_id(&self, id: &str) -> Result<Milestone>;
    async fn list_for_project(
        &self,
        project_id: &str,
        status: Option<MilestoneStatus>,
        after_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Milestone>>;
}

pub trait SprintRepository {
    async fn create(&self, new: NewSprint) -> Result<Sprint>;
    async fn find_by_id(&self, id: &str) -> Result<Sprint>;
    async fn find_active_for_project(&self, project_id: &str) -> Result<Option<Sprint>>;
    async fn list_for_project(
        &self,
        project_id: &str,
        status: Option<SprintStatus>,
        after_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Sprint>>;
}

pub trait TaskRepository {
    async fn create(&self, new: NewTask) -> Result<Task>;
    async fn find_by_id(&self, id: &str) -> Result<Task>;
    async fn list_for_project(
        &self,
        project_id: &str,
        filters: TaskFilters,
        after_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Task>>;
    async fn update(&self, id: &str, patch: UpdateTask) -> Result<Task>;
    async fn update_status(&self, id: &str, status: TaskStatus) -> Result<Task>;
}

pub trait TaskDependencyRepository {
    async fn add(&self, new: NewTaskDependency) -> Result<TaskDependency>;
    async fn remove(&self, from_id: &str, to_id: &str) -> Result<()>;
    async fn list_for_task(&self, task_id: &str) -> Result<Vec<TaskDependency>>;
}

pub trait SprintTaskRepository {
    async fn add(&self, new: NewSprintTask) -> Result<SprintTask>;
    async fn soft_remove(&self, sprint_id: &str, task_id: &str) -> Result<()>;
    async fn list_for_sprint(&self, sprint_id: &str) -> Result<Vec<SprintTask>>;
    async fn list_for_task(&self, task_id: &str) -> Result<Vec<SprintTask>>;
}
