mod helpers;

use roadboard_core::{
    planning::{NewTask, TaskFilters, TaskPriority, TaskRepository, TaskStatus, UpdateTask},
    project::{NewProject, ProjectRepository},
    user::{NewUser, UserRepository},
};
use roadboard_storage::{SqliteProjectRepository, SqliteTaskRepository, SqliteUserRepository};

async fn setup(pool: &sqlx::SqlitePool) -> (String, String) {
    let u = SqliteUserRepository(pool.clone())
        .create(NewUser {
            username: "tuser".to_string(),
            email: "tuser@test.com".to_string(),
            display_name: "T".to_string(),
            password_hash: "x".to_string(),
        })
        .await
        .unwrap();
    let p = SqliteProjectRepository(pool.clone())
        .create(NewProject {
            slug: "tproj".to_string(),
            name: "T Proj".to_string(),
            description: None,
            owner_user_id: u.id.clone(),
        })
        .await
        .unwrap();
    (u.id, p.id)
}

fn new_task(project_id: &str, user_id: &str, title: &str) -> NewTask {
    NewTask {
        project_id: project_id.to_string(),
        milestone_id: None,
        title: title.to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: TaskPriority::Medium,
        assignee_user_id: None,
        estimate: None,
        due_date: None,
        created_by_user_id: user_id.to_string(),
        updated_by_user_id: user_id.to_string(),
    }
}

#[tokio::test]
async fn create_and_update_title() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let repo = SqliteTaskRepository(pool);

    let task = repo.create(new_task(&project_id, &user_id, "original")).await.unwrap();
    let updated = repo
        .update(
            &task.id,
            UpdateTask {
                title: Some("renamed".to_string()),
                description: None,
                milestone_id: None,
                priority: None,
                assignee_user_id: None,
                due_date: None,
                estimate: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.title, "renamed");
}

#[tokio::test]
async fn update_status_to_done_sets_completed_at() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let repo = SqliteTaskRepository(pool);

    let task = repo.create(new_task(&project_id, &user_id, "todo")).await.unwrap();
    let done = repo.update_status(&task.id, TaskStatus::Done).await.unwrap();
    assert_eq!(done.status, TaskStatus::Done);
    assert!(done.completed_at.is_some());
}

#[tokio::test]
async fn update_status_from_done_clears_completed_at() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let repo = SqliteTaskRepository(pool);

    let task = repo.create(new_task(&project_id, &user_id, "reopen")).await.unwrap();
    repo.update_status(&task.id, TaskStatus::Done).await.unwrap();
    let in_progress = repo.update_status(&task.id, TaskStatus::InProgress).await.unwrap();
    assert!(in_progress.completed_at.is_none());
}

#[tokio::test]
async fn list_for_project_with_filters() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let repo = SqliteTaskRepository(pool);

    repo.create(new_task(&project_id, &user_id, "t1")).await.unwrap();
    let t2 = repo.create(new_task(&project_id, &user_id, "t2")).await.unwrap();
    repo.update_status(&t2.id, TaskStatus::Done).await.unwrap();

    let done_tasks = repo
        .list_for_project(
            &project_id,
            TaskFilters {
                status: Some(TaskStatus::Done),
                milestone_id: None,
                sprint_id: None,
                assignee_user_id: None,
            },
            None,
            10,
        )
        .await
        .unwrap();
    assert_eq!(done_tasks.len(), 1);
}
