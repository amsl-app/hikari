mod common;

use crate::common::setup_schema;
use crate::common::user::create_test_user;
use chrono::Weekday;
use hikari_db::user;
use hikari_entity::user::{ActiveModel as ActiveUserModel, Entity as User, Gender};
use paste::paste;
use sea_orm::ActiveValue::{Set, Unchanged};
use sea_orm::{ConnectionTrait, Database, DbBackend, EntityTrait, FromQueryResult, JsonValue, Statement};

use test_log::test;

use uuid::Uuid;

#[test(tokio::test)]
async fn test_change_user() {
    let db = &Database::connect("sqlite::memory:").await.unwrap();

    setup_schema(db).await.unwrap();
    let user = create_test_user(db).await;

    let users = User::find().all(db).await.unwrap();
    assert_eq!(users[0].id, user.id);

    user::Mutation::update_user(
        db,
        ActiveUserModel {
            id: Unchanged(user.id),
            semester: Set(Some(2)),
            ..ActiveUserModel::default()
        },
    )
    .await
    .unwrap();

    let user = user::Query::find_user_by_id(db, user.id).await.unwrap().unwrap();
    assert_eq!(user.semester, Some(2));
    assert_eq!(user.subject, Some("test".to_owned()));
}

macro_rules! test_set_user_field {
    ($i:ident, $v:expr_2021) => {
        paste! {
            #[test(tokio::test)]
            async fn [<test_update_user_ $i>]() {
                let db = &Database::connect("sqlite::memory:").await.unwrap();

                setup_schema(db).await.unwrap();

                let user = create_test_user(db).await;

                user::Mutation::[<update_user_ $i>](db, user.id, $v).await.unwrap();

                let user = user::Query::find_user_by_id(db, user.id).await.unwrap().unwrap();
                println!("Expected / Actual value: {:?} / {:?}", $v, &user.$i);
                assert_eq!(user.$i, ($v).into());
            }
        }
    };
}

test_set_user_field!(name, Some("Herbert".to_owned()));
test_set_user_field!(
    birthday,
    Some(chrono::NaiveDate::from_isoywd_opt(2023, 1, Weekday::Mon).unwrap())
);
test_set_user_field!(subject, Some("Pomology".to_owned()));
test_set_user_field!(semester, Some(1337));
test_set_user_field!(gender, Some(Gender::Male));
test_set_user_field!(current_module, Some("thinking".to_owned()));
test_set_user_field!(current_session, Some("about_birds".to_owned()));

#[test(tokio::test)]
async fn test_update_user_onboarding() {
    let db = &Database::connect("sqlite::memory:").await.unwrap();

    setup_schema(db).await.unwrap();

    let user = create_test_user(db).await;

    let user_id = user.id;
    user::Mutation::update_user_onboarding(db, user_id, true).await.unwrap();

    let user = user::Query::find_user_by_id(db, user_id).await.unwrap().unwrap();
    println!("Expected / Actual value: true / {:?}", &user.onboarding);
    assert!(user.onboarding);
}

#[test(tokio::test)]
async fn test_user_string_key() {
    let db = &Database::connect("sqlite::memory:").await.unwrap();

    setup_schema(db).await.unwrap();

    let user_id = Uuid::new_v4();

    db.execute_unprepared(&format!(
        "INSERT INTO users (id, name) VALUES ('{}', 'johnny')",
        user_id.as_hyphenated()
    ))
    .await
    .unwrap();
    let user = create_test_user(db).await;

    // let query = <(Option<String>,)>::find_by_statement::<ResultCol>(Statement::from_sql_and_values(
    //     DbBackend::Sqlite,
    //     r#"SELECT "user"."id" FROM user"#,
    //     [],
    // ));
    //
    // let res: Vec<(Option<String>,)> = query.all(db).await.unwrap();
    // let res = db.execute_unprepared(&format!(
    //     "SELECT id, name FROM user"
    // )).await.unwrap();
    let res: Vec<JsonValue> = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            r#"SELECT "users"."id", "users"."name" FROM "users""#,
            [],
        ))
        .await
        .unwrap()
        .into_iter()
        .map(|qr| FromQueryResult::from_query_result(&qr, "").unwrap())
        .collect();
    println!("Users: {res:?}");
    assert_eq!(res.len(), 2);
    let user_name = user::Query::find_user_by_id(db, user.id).await.unwrap().unwrap().name;
    assert_eq!(user_name, None);

    assert!(
        user::Query::find_user_by_id(db, user_id).await.unwrap().is_none(),
        "Inserting Ids manually should have a different format"
    );
    println!("Expected / Actual value: johnny / {:?}", &user_name);
}
