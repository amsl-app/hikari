use hikari_entity::module::session::status::{Model, Status};
use hikari_entity::module::session::{status, status::Entity as Instance, status::Model as InstanceModel};
use sea_orm::prelude::*;
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_for_user<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module: &str,
        session: &str,
    ) -> Result<Option<InstanceModel>, DbErr> {
        let res = Instance::find()
            .filter(status::Column::UserId.eq(user_id))
            .filter(status::Column::Module.eq(module))
            .filter(status::Column::Session.eq(session))
            .one(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, %module, %session, "failed to get session status");
        })
    }

    pub async fn get_finished_sessions<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module: &str,
    ) -> Result<Vec<InstanceModel>, DbErr> {
        let res = Instance::find()
            .filter(status::Column::UserId.eq(user_id))
            .filter(status::Column::Module.eq(module))
            .filter(status::Column::Status.eq(Status::Finished))
            .all(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, %module, "failed to get finished sessions");
        })
    }

    pub async fn for_module<C: ConnectionTrait>(conn: &C, user_id: Uuid, module: &str) -> Result<Vec<Model>, DbErr> {
        let res = Instance::find()
            .filter(status::Column::UserId.eq(user_id))
            .filter(status::Column::Module.eq(module))
            .all(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, "failed to get module session");
        })
    }

    pub async fn find_other_running_sessions<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module: &str,
        session: &str,
    ) -> Result<Vec<Model>, DbErr> {
        let res = Instance::find()
            .filter(status::Column::UserId.eq(user_id))
            .filter(status::Column::Module.eq(module))
            .filter(status::Column::Session.ne(session))
            .filter(status::Column::Status.eq(Status::Started))
            .all(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, "failed to get other running sessions");
        })
    }

    pub async fn all<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<Vec<Model>, DbErr> {
        let res = Instance::find()
            .filter(status::Column::UserId.eq(user_id))
            .all(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, "failed to get sessions");
        })
    }
}
