use hikari_entity::oidc_mapping::{ActiveModel, Entity, Model};
use sea_orm::ActiveValue::Set;
use sea_orm::TransactionTrait;
use sea_orm::prelude::*;

pub struct Mutation;

impl Mutation {
    pub async fn create_oidc_mapping<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        oidc_sub: String,
    ) -> Result<Model, DbErr> {
        let oidc_mapping = ActiveModel {
            user_id: Set(user_id),
            oidc_sub: Set(oidc_sub.clone()),
            ..Default::default()
        };

        let mapping = Entity::insert(oidc_mapping).exec_with_returning(conn).await?;
        Ok(mapping)
    }
}
