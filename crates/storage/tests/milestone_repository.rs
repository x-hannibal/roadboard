mod helpers;

use roadboard_core::{
    planning::{MilestoneRepository, MilestoneStatus, NewMilestone},
    project::{NewProject, ProjectRepository},
    user::{NewUser, UserRepository},
};
use roadboard_storage::{SqliteMilestoneRepository, SqliteProjectRepository, SqliteUserRepository};

async fn setup(pool: &sqlx::SqlitePool) -> (String, String) {
    let u = SqliteUserRepository(pool.clone())
        .create(NewUser {
            username: "muser".to_string(),
            email: "muser@test.com".to_string(),
            display_name: "M".to_string(),
            password_hash: "x".to_string(),
        })
        .await
        .unwrap();
    let p = SqliteProjectRepository(pool.clone())
        .create(NewProject {
            slug: "mproj".to_string(),
            name: "M Proj".to_string(),
            description: None,
            owner_user_id: u.id.clone(),
        })
        .await
        .unwrap();
    (u.id, p.id)
}

#[tokio::test]
async fn list_without_filter_returns_all() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let repo = SqliteMilestoneRepository(pool);

    for slug in ["m1", "m2"] {
        repo.create(NewMilestone {
            project_id: project_id.clone(),
            title: slug.to_string(),
            description: None,
            due_date: None,
            status: MilestoneStatus::Planned,
            order_index: 0,
            created_by_user_id: user_id.clone(),
            updated_by_user_id: user_id.clone(),
        })
        .await
        .unwrap();
    }

    let all = repo.list_for_project(&project_id, None, None, 10).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn list_with_status_filter() {
    let pool = helpers::setup_db().await;
    let (user_id, project_id) = setup(&pool).await;
    let repo = SqliteMilestoneRepository(pool);

    repo.create(NewMilestone {
        project_id: project_id.clone(),
        title: "planned".to_string(),
        description: None,
        due_date: None,
        status: MilestoneStatus::Planned,
        order_index: 0,
        created_by_user_id: user_id.clone(),
        updated_by_user_id: user_id.clone(),
    })
    .await
    .unwrap();

    repo.create(NewMilestone {
        project_id: project_id.clone(),
        title: "done".to_string(),
        description: None,
        due_date: None,
        status: MilestoneStatus::Done,
        order_index: 1,
        created_by_user_id: user_id.clone(),
        updated_by_user_id: user_id.clone(),
    })
    .await
    .unwrap();

    let planned = repo
        .list_for_project(&project_id, Some(MilestoneStatus::Planned), None, 10)
        .await
        .unwrap();
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].title, "planned");
}
