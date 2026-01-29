mod common;

use crate::common::setup_schema;
use crate::common::user::create_test_user;
use hikari_db::user_handle::{HandleGenerator, Mutation, RandomHandleGenerator};
use sea_orm::{ConnectionTrait, Database, Statement};
use uuid::Uuid;

use test_log::test;

struct ConstantHandleGenerator;

impl HandleGenerator for ConstantHandleGenerator {
    fn generate_handle(len: usize) -> Vec<u8> {
        vec![0; len]
    }
}

#[test(tokio::test)]
async fn test_handle_creation() {
    let db = &Database::connect("sqlite::memory:").await.unwrap();

    setup_schema(db).await.unwrap();

    let user_id = Uuid::new_v4();

    db.execute_unprepared(&format!(
        "INSERT INTO users (id, name) VALUES ('{}', 'johnny')",
        user_id.as_hyphenated()
    ))
    .await
    .unwrap();
    let user_a = create_test_user(db).await;
    let user_b = create_test_user(db).await;
    let user_c = create_test_user(db).await;

    let user_handle_c_1 = Mutation::get_or_create_handle::<RandomHandleGenerator, _>(db, user_c.id, 5)
        .await
        .unwrap();
    let user_handle_a_1 = Mutation::get_or_create_handle::<ConstantHandleGenerator, _>(db, user_a.id, 5)
        .await
        .unwrap();
    let user_handle_a_2 = Mutation::get_or_create_handle::<ConstantHandleGenerator, _>(db, user_a.id, 5)
        .await
        .unwrap();
    let user_handle_b_1 = Mutation::get_or_create_handle::<ConstantHandleGenerator, _>(db, user_b.id, 5)
        .await
        .unwrap();

    let res = db
        .query_all(Statement::from_string(
            db.get_database_backend(),
            "SELECT * FROM user_handle",
        ))
        .await
        .unwrap();
    let first = res.first().unwrap();
    let handle: Vec<u8> = first.try_get("", "handle").unwrap();
    let user_id: Uuid = first.try_get("", "user_id").unwrap();
    println!("{handle:?} {user_id:?}");

    assert_eq!(user_handle_a_1, user_handle_a_2);
    assert_ne!(user_handle_a_1, user_handle_b_1);
    assert_ne!(user_handle_a_1, user_handle_c_1);
    assert_ne!(user_handle_a_1.handle.len(), user_handle_b_1.handle.len());
}
