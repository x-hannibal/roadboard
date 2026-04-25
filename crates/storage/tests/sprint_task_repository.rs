mod helpers;

use chrono::NaiveDate;
use roadboard_core::{
    planning::{
        NewSprint, NewSprintTask, NewTask, SprintRepository, SprintStatus, SprintTaskRepository,
        TaskPriority, TaskRepository, TaskStatus,
    },
    project::{NewProject, ProjectRepository},
    user::{NewUser, UserRepository},
};
use roadboard_storage::{
    SqliteProjectRepository, SqliteSprintRepository, SqliteSprintTaskRepository,
    SqliteTaskRepository, SqliteUserRepository,
};

async fn setup(pool: &sqlx::SqlitePool) -> (String, String, String, String) {
    let u = SqliteUserRepository(pool.clone())
        .create(NewUser {
            username: "stuser".to_string(),
            email: "stuser@test.com".to_string(),
            display_name: "ST".to_string(),
            password_hash: "x".to_string(),
        })
        .await
        .unwrap();
    let p = SqliteProjectRepository(pool.clone())
        .create(NewProject {
            slug: "stproj".to_string(),
            name: "ST Proj".to_string(),
            description: None,
            owner_user_id: u.id.clone(),
        })
        .await
        .unwrap();
    let s = SqliteSprintRepository(pool.clone())
        .create(NewSprint {
            project_id: p.id.clone(),
            name: "Sprint 1".to_string(),
            goal: None,
            start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 1, 14).unwrap(),
            status: SprintStatus::Active,
            created_by_user_id: u.id.clone(),
            updated_by_user_id: u.id.clone(),
        })
        .await
        .unwrap();
    let t = SqliteTaskRepository(pool.clone())
        .create(NewTask {
            project_id: p.id.clone(),
            milestone_id: None,
            title: "Task".to_string(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            assignee_user_id: None,
            estimate: None,
            due_date: None,
            created_by_user_id: u.id.clone(),
            updated_by_user_id: u.id.clone(),
        })
        .await
        .unwrap();
    (u.id, p.id, s.id, t.id)
}

#[tokio::test]
async fn add_and_list_for_sprint() {
    let pool = helpers::setup_db().await;
    let (_user_id, _project_id, sprint_id, task_id) = setup(&pool).await;
    let repo = SqliteSprintTaskRepository(pool);

    repo.add(NewSprintTask {
        sprint_id: sprint_id.clone(),
        task_id: task_id.clone(),
        carried_from_sprint_id: None,
    })
    .await
    .unwrap();

    let items = repo.list_for_sprint(&sprint_id).await.unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].removed_at.is_none());
}

#[tokio::test]
async fn soft_remove_and_readd_clears_removed_at() {
    let pool = helpers::setup_db().await;
    let (_user_id, _project_id, sprint_id, task_id) = setup(&pool).await;
    let repo = SqliteSprintTaskRepository(pool);

    repo.add(NewSprintTask {
        sprint_id: sprint_id.clone(),
        task_id: task_id.clone(),
        carried_from_sprint_id: None,
    })
    .await
    .unwrap();

    repo.soft_remove(&sprint_id, &task_id).await.unwrap();

    let after_remove = repo.list_for_sprint(&sprint_id).await.unwrap();
    assert!(after_remove[0].removed_at.is_some());

    repo.add(NewSprintTask {
        sprint_id: sprint_id.clone(),
        task_id: task_id.clone(),
        carried_from_sprint_id: None,
    })
    .await
    .unwrap();

    let after_readd = repo.list_for_sprint(&sprint_id).await.unwrap();
    assert_eq!(after_readd.len(), 1);
    assert!(after_readd[0].removed_at.is_none());
}
