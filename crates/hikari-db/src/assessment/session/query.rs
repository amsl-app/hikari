use crate::util::RequireRecord;
use hikari_entity::assessment::session;
use hikari_entity::assessment::session::{Entity as SessionEntity, Model as Session};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn load_session<C: ConnectionTrait>(conn: &C, user_id: Uuid, session_id: Uuid) -> Result<Session, DbErr> {
        SessionEntity::find_by_id(session_id)
            .filter(session::Column::UserId.eq(user_id))
            .one(conn)
            .await
            .require()
            .inspect_err(
                |error| tracing::error!(error = error as &dyn Error, %user_id, %session_id, "failed to load session"),
            )
    }

    pub async fn load_sessions<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<Vec<Session>, DbErr> {
        SessionEntity::find()
            .filter(session::Column::UserId.eq(user_id))
            .all(conn)
            .await
            .inspect_err(|error| tracing::error!(error = error as &dyn Error, %user_id, "failed to load sessions"))
    }
}
