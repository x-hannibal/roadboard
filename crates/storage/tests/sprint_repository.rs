mod helpers;

use chrono::NaiveDate;
use roadboard_core::{
    planning::{NewSprint, SprintRepository, SprintStatus},
    project::{NewProject, ProjectRepository},
    user::{NewUser, UserRepository},
};
use roadboard_storage::{SqliteProjectRepository, SqliteSprintRepository, SqliteUserRepository};

async fn setup(pool: &sqlx::SqlitePool) -> (String, String) {
    let u = SqliteUserRepository(pool.clone())
        .create(NewUser {
            username: "suser".to_string(),
            email: "suser@test.com".to_string(),
            display_name: "S".to_string(),
            password_hash: "x".to_string(),
        })
        .await
        .unwrap();
    let p = SqliteProjectRepository(pool.clone())
        .create(NewProject {
            slug: "sproj".to_string(),
            name: "S Proj".to_string(),
            description: None,
            owner_user_id: u.id.clone(),
        })
        .await
        .unwrap();
    (u.id, p.id)
}

fn sprint(project_id: &str, user_id: &str, status: SprintStatus, name: &str) -> NewSprint {
    NewSprint {
        project_id: project_id.to_string(),
        name: name.to_string(),
        goal: None,
        start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2026, 1, 14).unwrap(),
        status,
        created_by_user_id: user_id.to_string(),
        updated_by_user_id: user_id.to_string(),
    }
}

#[tokio::test]
async fn find_active_for_project() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let repo = SqliteSprintRepository(pool);

    let active = repo
        .create(sprint(&project_id, &user_id, SprintStatus::Active, "S1"))
        .await
        .unwrap();
    repo.create(sprint(&project_id, &user_id, SprintStatus::Planned, "S2"))
        .await
        .unwrap();

    let found = repo.find_active_for_project(&project_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, active.id);
}

#[tokio::test]
async fn no_active_sprint_returns_none() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let repo = SqliteSprintRepository(pool);

    repo.create(sprint(&project_id, &user_id, SprintStatus::Planned, "S3"))
        .await
        .unwrap();

    let found = repo.find_active_for_project(&project_id).await.unwrap();
    assert!(found.is_none());
}
