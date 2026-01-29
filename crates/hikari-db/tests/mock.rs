use hikari_db::config::Query;
use hikari_entity::config;
use sea_orm::Value::String;
use sea_orm::{DatabaseBackend, DbErr, MockDatabase};
use std::collections::BTreeMap;
use test_log::test;
use uuid::Uuid;

#[test(tokio::test)]
async fn test_get_config_value() -> Result<(), DbErr> {
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results([[BTreeMap::from([("value", String(Some(Box::new("val1".to_owned()))))])]])
        .into_connection();

    assert_eq!(
        Query::get_config_value(&db, Uuid::new_v4(), "edf").await?,
        Some("val1".to_owned())
    );

    Ok(())
}

#[tokio::test]
async fn test_get_config() -> Result<(), DbErr> {
    let user_id = Uuid::new_v4();
    let models = [
        config::Model {
            user_id,
            key: "k".to_owned(),
            value: "v".to_owned(),
        },
        config::Model {
            user_id,
            key: "x".to_owned(),
            value: "y".to_owned(),
        },
    ];
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results([models.clone()])
        .into_connection();

    assert_eq!(Query::get_user_config(&db, user_id).await?, Vec::from(models));

    Ok(())
}
