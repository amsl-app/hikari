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
        let new_groups = groups
            .iter()
            .map(|group| ActiveModel {
                user_id: user_id.into_active_value(),
                value: group.clone().into_active_value(),
            })
            .collect::<Vec<_>>();

        db.transaction(|conn| {
            Box::pin(async move {
                if !new_groups.is_empty() {
                    Entity::insert_many(new_groups)
                        .on_conflict_do_nothing()
                        .exec(conn)
                        .await?;
                }
                Entity::delete_many()
                    .filter(oidc_groups::Column::UserId.eq(user_id))
                    .filter(oidc_groups::Column::Value.is_not_in(groups))
                    .exec(conn)
                    .await?;
                Result::<_, DbErr>::Ok(())
            })
        })
        .await
        .flatten_res()?;
        Ok(())
    }
}
