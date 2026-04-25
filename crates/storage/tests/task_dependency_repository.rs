mod helpers;

use roadboard_core::{
    planning::{
        DependencyType, NewTask, NewTaskDependency, TaskDependencyRepository, TaskPriority,
        TaskRepository, TaskStatus,
    },
    project::{NewProject, ProjectRepository},
    user::{NewUser, UserRepository},
    Error,
};
use roadboard_storage::{
    SqliteProjectRepository, SqliteTaskDependencyRepository, SqliteTaskRepository,
    SqliteUserRepository,
};

async fn make_task(
    pool: &sqlx::SqlitePool,
    project_id: &str,
    user_id: &str,
    title: &str,
) -> String {
    SqliteTaskRepository(pool.clone())
        .create(NewTask {
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
        })
        .await
        .unwrap()
        .id
}

async fn setup(pool: &sqlx::SqlitePool) -> (String, String) {
    let u = SqliteUserRepository(pool.clone())
        .create(NewUser {
            username: "duser".to_string(),
            email: "duser@test.com".to_string(),
            display_name: "D".to_string(),
            password_hash: "x".to_string(),
        })
        .await
        .unwrap();
    let p = SqliteProjectRepository(pool.clone())
        .create(NewProject {
            slug: "dproj".to_string(),
            name: "D Proj".to_string(),
            description: None,
            owner_user_id: u.id.clone(),
        })
        .await
        .unwrap();
    (u.id, p.id)
}

#[tokio::test]
async fn add_and_list_dependency() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let t1 = make_task(&pool, &project_id, &user_id, "T1").await;
    let t2 = make_task(&pool, &project_id, &user_id, "T2").await;
    let dep_repo = SqliteTaskDependencyRepository(pool);

    dep_repo
        .add(NewTaskDependency {
            from_task_id: t1.clone(),
            to_task_id: t2.clone(),
            dependency_type: DependencyType::Blocks,
        })
        .await
        .unwrap();

    let deps = dep_repo.list_for_task(&t1).await.unwrap();
    assert_eq!(deps.len(), 1);
}

#[tokio::test]
async fn self_loop_returns_invalid_input() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let t1 = make_task(&pool, &project_id, &user_id, "Loop").await;
    let dep_repo = SqliteTaskDependencyRepository(pool);

    let err = dep_repo
        .add(NewTaskDependency {
            from_task_id: t1.clone(),
            to_task_id: t1.clone(),
            dependency_type: DependencyType::Blocks,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidInput(_)));
}
