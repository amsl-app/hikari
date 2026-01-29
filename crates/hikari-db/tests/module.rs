mod common;

use crate::common::setup_schema;
use crate::common::user::create_test_user;

use sea_orm::Database;
use test_log::test;
use uuid::Uuid;

#[test(tokio::test)]
async fn test_assessment_module_creation() {
    let conn = &Database::connect("sqlite::memory:").await.unwrap();

    setup_schema(conn).await.unwrap();

    let user = create_test_user(conn).await;

    hikari_db::module::assessment::Mutation::insert_or_update_module_assessment(
        conn,
        user.id,
        "test".to_owned(),
        Some(Uuid::new_v4()),
        None,
    )
    .await
    .expect_err("Should fail because the assessment session does not exist");
    let assessment = hikari_db::assessment::session::Mutation::new_assessment(conn, user.id, "test".to_string())
        .await
        .unwrap();
    let assessments = hikari_db::assessment::session::Query::load_sessions(conn, user.id)
        .await
        .unwrap();
    assert_eq!(assessments[0].id, assessment.id);
    println!("Assessment id: {}", assessment.id);
    let module_assessment = hikari_db::module::assessment::Mutation::insert_or_update_module_assessment(
        conn,
        user.id,
        "test".to_owned(),
        Some(assessment.id),
        None,
    )
    .await
    .unwrap();
    assert_eq!(module_assessment.last_pre.unwrap(), assessment.id);
    assert!(module_assessment.last_post.is_none());
}
