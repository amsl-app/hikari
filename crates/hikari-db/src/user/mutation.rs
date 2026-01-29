use hikari_entity::user::{ActiveModel, Entity, Gender, Model};
use paste::paste;
use sea_orm::ActiveValue::{Set, Unchanged};
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel};
use std::error::Error;
use uuid::Uuid;

pub struct Mutation;

macro_rules! update_user_field {
    ($i:ident, $t:ty) => {
        paste! {
            pub async fn [<update_user_ $i>]<C: ConnectionTrait>(conn: &C, user_id: Uuid, $i: Option<$t>) -> Result<Model, DbErr> {
                let user = ActiveModel {
                    id: ActiveValue::Unchanged(user_id),
                    $i: ActiveValue::Set($i),
                    ..<hikari_entity::user::ActiveModel as std::default::Default>::default()
                };
                user.update(conn).await
            }
        }
    };
}

impl Mutation {
    pub async fn create_user<C: ConnectionTrait>(conn: &C) -> Result<Model, DbErr> {
        let new_user = ActiveModel {
            id: Set(Uuid::new_v4()),
            ..Default::default()
        };

        let user: Model = Entity::insert(new_user.into_active_model())
            .exec_with_returning(conn)
            .await?;

        Ok(user)
    }

    pub async fn update_user<C: ConnectionTrait>(conn: &C, user: ActiveModel) -> Result<Model, DbErr> {
        user.update(conn).await
    }

    update_user_field!(name, String);
    update_user_field!(birthday, chrono::NaiveDate);
    update_user_field!(subject, String);
    update_user_field!(semester, i16);
    update_user_field!(gender, Gender);
    update_user_field!(current_module, String);
    update_user_field!(current_session, String);

    pub async fn update_user_onboarding<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        onboarding: bool,
    ) -> Result<Model, DbErr> {
        let user = ActiveModel {
            id: Unchanged(user_id),
            onboarding: Set(onboarding),
            ..Default::default()
        };
        user.update(conn).await
    }

    pub async fn delete<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<(), DbErr> {
        let res = Entity::delete_by_id(user_id).exec(conn).await;
        if let Err(error) = res {
            tracing::error!(error = &error as &dyn Error, "failed to delete user");
            return Err(error);
        }
        Ok(())
    }
}
