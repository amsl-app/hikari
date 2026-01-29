use hikari_entity::module::status::Status;
use hikari_entity::module::status::{self, Entity as ModuleStatusEntity, Model as ModuleStatus};
use sea_orm::prelude::*;
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_for_user<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module: &str,
    ) -> Result<Option<ModuleStatus>, DbErr> {
        let res = ModuleStatusEntity::find()
            .filter(status::Column::UserId.eq(user_id))
            .filter(status::Column::Module.eq(module))
            .one(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, %module, "failed to get module status");
        })
    }

    pub async fn get_finished_modules<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<Vec<ModuleStatus>, DbErr> {
        let res = ModuleStatusEntity::find()
            .filter(status::Column::UserId.eq(user_id))
            .filter(status::Column::Status.eq(Status::Finished))
            .all(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, "failed to get finished modules");
        })
    }

    pub async fn find_other_running<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module: &str,
    ) -> Result<Vec<ModuleStatus>, DbErr> {
        let res = ModuleStatusEntity::find()
            .filter(status::Column::UserId.eq(user_id))
            .filter(status::Column::Module.ne(module))
            .filter(status::Column::Status.eq(Status::Started))
            .all(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, "failed to get other module session instances");
        })
    }

    pub async fn all<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<Vec<ModuleStatus>, DbErr> {
        let res = ModuleStatusEntity::find()
            .filter(status::Column::UserId.eq(user_id))
            .all(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, "failed to load modules");
        })
    }
}
