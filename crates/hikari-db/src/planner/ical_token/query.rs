use hikari_entity::planner_ical_token::{Column, Entity as IcalToken};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn find_by_token<C: ConnectionTrait>(db: &C, token: &str) -> Result<Option<Uuid>, DbErr> {
        let row = IcalToken::find()
            .filter(Column::Token.eq(token))
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = %error, "failed to find ical token");
            })?;
        Ok(row.map(|m| m.user_id))
    }
}
