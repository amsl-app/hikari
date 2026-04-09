use hikari_entity::oidc_groups;
use hikari_entity::oidc_groups::{ActiveModel, Entity};
use sea_orm::sea_query::OnConflict;
use std::collections::HashSet;

use crate::util::FlattenTransactionResultExt;
use sea_orm::prelude::*;
use sea_orm::{IntoActiveValue, TransactionTrait};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn set<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: Uuid,
        groups: HashSet<String>,
    ) -> Result<(), DbErr> {
        let groups = groups
            .into_iter()
            .map(|group| ActiveModel {
                user_id: user_id.into_active_value(),
                value: group.into_active_value(),
            })
            .collect::<Vec<_>>();

        let on_conflict = OnConflict::columns([oidc_groups::Column::UserId, oidc_groups::Column::Value])
            .update_column(oidc_groups::Column::Value)
            .to_owned();

        db.transaction(|conn| {
            Box::pin(async move {
                Entity::delete_many()
                    .filter(oidc_groups::Column::UserId.eq(user_id))
                    .exec(conn)
                    .await?;
                if !groups.is_empty() {
                    Entity::insert_many(groups).on_conflict(on_conflict).exec(conn).await?;
                }
                Result::<_, DbErr>::Ok(())
            })
        })
        .await
        .flatten_res()?;
        Ok(())
    }
}
