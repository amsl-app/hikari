use hikari_entity::user::{Entity as User, Model as UserModel};
use sea_orm::{DatabaseConnection, EntityTrait, IntoActiveModel};
use uuid::Uuid;

#[allow(dead_code)]
pub async fn create_test_user(db: &DatabaseConnection) -> UserModel {
    let id = Uuid::new_v4();
    let user = UserModel {
        id,
        name: None,
        birthday: None,
        subject: Some("test".to_owned()),
        semester: Some(5),
        gender: None,
        current_module: None,
        current_session: None,
        onboarding: false,
    };
    User::insert(user.clone().into_active_model()).exec(db).await.unwrap();
    user
}
