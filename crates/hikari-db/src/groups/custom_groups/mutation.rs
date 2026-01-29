use crate::util::FlattenTransactionResultExt;
use hikari_entity::custom_groups;
use hikari_entity::custom_groups::{ActiveModel, Entity};
use sea_orm::prelude::*;
use sea_orm::{IntoActiveValue, TransactionTrait};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn add<C: ConnectionTrait + TransactionTrait>(db: &C, user_id: Uuid, group: String) -> Result<(), DbErr> {
        let group = ActiveModel {
            user_id: user_id.into_active_value(),
            value: group.into_active_value(),
        };
        // ensure that the group does not already exist
        let on_conflict =
            sea_orm::sea_query::OnConflict::columns([custom_groups::Column::UserId, custom_groups::Column::Value])
                .update_column(custom_groups::Column::Value)
                .clone();

        db.transaction(|conn| {
            Box::pin(async move {
                Entity::insert(group).on_conflict(on_conflict).exec(conn).await?;
                Result::<_, DbErr>::Ok(())
            })
        })
        .await
        .flatten_res()?;
        Ok(())
    }
}
