use crate::common::setup_schema;
use hikari_db::config;
use sea_orm::{ConnectionTrait, Database, Statement};
use test_log::test;
use uuid::Uuid;

mod common;

#[test(tokio::test)]
async fn test_config() {
    let db = Database::connect("sqlite::memory:").await.unwrap();

    // Setup database schema
    setup_schema(&db).await.unwrap();
    let user_id = Uuid::new_v4();
    let user_b_id = Uuid::new_v4();

    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO users (id) VALUES ($1)",
        vec![user_id.into()],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO users (id) VALUES ($1)",
        vec![user_b_id.into()],
    ))
    .await
    .unwrap();

    config::Mutation::set_config_value(&db, user_id, "key1".to_owned(), "value-a".to_owned())
        .await
        .unwrap();
    config::Mutation::set_config_value(&db, user_id, "key2".to_owned(), "value-b".to_owned())
        .await
        .unwrap();
    config::Mutation::set_config_value(&db, user_id, "key3".to_owned(), "value-c".to_owned())
        .await
        .unwrap();
    config::Mutation::set_config_value(&db, user_id, "key3".to_owned(), "value-d".to_owned())
        .await
        .unwrap();

    config::Mutation::set_config_value(&db, user_b_id, "key4".to_owned(), "value-e".to_owned())
        .await
        .unwrap();

    config::Mutation::delete_config_value(&db, user_id, "key1".to_owned())
        .await
        .unwrap();

    let conf_a = config::Query::get_user_config(&db, user_id).await.unwrap();

    let mut conf_a: Vec<_> = conf_a.into_iter().map(|model| (model.key, model.value)).collect();
    conf_a.sort_by_key(|val| val.0.clone());

    assert_eq!(
        conf_a.as_slice(),
        [("key2", "value-b"), ("key3", "value-d"),]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect::<Vec<_>>()
    );
}
