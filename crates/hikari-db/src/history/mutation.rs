use chrono::Utc;
use sea_orm::IntoActiveValue;
use sea_orm::prelude::*;
use uuid::Uuid;

pub struct Mutation;
use hikari_entity::history::{ActiveModel as ActiveHistoryModel, Model as HistoryModel};

impl Mutation {
    pub async fn create<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<HistoryModel, DbErr> {
        let history_id = Uuid::new_v4();
        let history = ActiveHistoryModel {
            id: history_id.into_active_value(),
            user_id: user_id.into_active_value(),
            completed: Utc::now().naive_utc().into_active_value(),
        };

        history.insert(conn).await
    }
}
