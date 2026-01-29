use hikari_entity::oidc_groups;
use hikari_entity::oidc_groups::{ActiveModel, Entity};
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

        db.transaction(|conn| {
            Box::pin(async move {
                Entity::delete_many()
                    .filter(oidc_groups::Column::UserId.eq(user_id))
                    .exec(conn)
                    .await?;
                if !groups.is_empty() {
                    Entity::insert_many(groups).exec(conn).await?;
                }
                Result::<_, DbErr>::Ok(())
            })
        })
        .await
        .flatten_res()?;
        Ok(())
    }
}
