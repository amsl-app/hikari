use chrono::Utc;
use hikari_entity::groups_token::{ActiveModel, Column, Entity};

use sea_orm::{ConnectionTrait, DbErr, EntityTrait, IntoActiveValue, TransactionTrait};
use uuid::Uuid;

use crate::util::FlattenTransactionResultExt;

pub struct Mutation;

impl Mutation {
    pub async fn add<C: ConnectionTrait + TransactionTrait>(db: &C, user_id: Uuid, token: String) -> Result<(), DbErr> {
        let group_token = ActiveModel {
            user_id: user_id.into_active_value(),
            token: token.into_active_value(),
            added_at: Utc::now().naive_utc().into_active_value(),
        };
        // ensure that the group does not already exist
        let on_conflict = sea_orm::sea_query::OnConflict::columns([Column::UserId, Column::Token])
            .do_nothing()
            .clone();

        db.transaction(|conn| {
            Box::pin(async move {
                Entity::insert(group_token).on_conflict(on_conflict).exec(conn).await?;
                Result::<_, DbErr>::Ok(())
            })
        })
        .await
        .flatten_res()?;
        Ok(())
    }
}
