use crate::util::RequireRecord;
use hikari_entity::assessment::session;
use hikari_entity::assessment::session::{AssessmentStatus, Entity as SessionEntity, Model as Session};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};
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

    pub async fn load_first_session<C: ConnectionTrait>(
        conn: &C,
        assessment: &str,
        user_id: Uuid,
    ) -> Result<Option<Session>, DbErr> {
        SessionEntity::find()
            .filter(session::Column::UserId.eq(user_id))
            .filter(session::Column::Assessment.eq(assessment))
            .filter(session::Column::Completed.is_not_null())
            .order_by(session::Column::Completed, sea_orm::Order::Asc)
            .one(conn)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, %user_id, %assessment, "failed to load first session")
            })
    }

    pub async fn load_last_session<C: ConnectionTrait>(
        conn: &C,
        assessment: &str,
        min_completed: Option<chrono::NaiveDateTime>,
        user_id: Uuid,
    ) -> Result<Option<Session>, DbErr> {
        let mut query = SessionEntity::find()
            .filter(session::Column::UserId.eq(user_id))
            .filter(session::Column::Assessment.eq(assessment))
            .filter(session::Column::Completed.is_not_null());

        if let Some(min_completed) = min_completed {
            query = query.filter(session::Column::Completed.gt(min_completed));
        }

        query.order_by(session::Column::Completed, sea_orm::Order::Desc)
            .one(conn)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, %user_id, %assessment, "failed to load last session")
            })
    }

    pub async fn load_running_session<C: ConnectionTrait>(
        conn: &C,
        assessment: &str,
        user_id: Uuid,
    ) -> Result<Option<Session>, DbErr> {
        SessionEntity::find()
            .filter(session::Column::UserId.eq(user_id))
            .filter(session::Column::Assessment.eq(assessment))
            .filter(session::Column::Status.eq(AssessmentStatus::Running))
            .one(conn)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, %user_id, %assessment, "failed to load running session")
            })
    }
}
