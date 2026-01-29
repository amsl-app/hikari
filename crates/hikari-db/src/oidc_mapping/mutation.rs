use hikari_entity::{
    oidc_mapping,
    oidc_mapping::{ActiveModel, Entity, Model},
};

use crate::util::FlattenTransactionResultExt;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::*;
use sea_orm::{TransactionTrait, sea_query};

pub struct Mutation;

impl Mutation {
    pub async fn create_oidc_mapping<C: TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        oidc_sub: String,
    ) -> Result<Model, DbErr> {
        let oidc_mapping = ActiveModel {
            user_id: Set(user_id),
            oidc_sub: Set(oidc_sub.clone()),
            ..Default::default()
        };

        conn.transaction(|txn| {
            Box::pin(async move {
                // Can't use returning because of how sea_orm works
                Entity::insert(oidc_mapping)
                    .on_conflict(
                        sea_query::OnConflict::column(oidc_mapping::Column::OidcSub)
                            .do_nothing()
                            .clone(),
                    )
                    .do_nothing()
                    .exec(txn)
                    .await?;
                let mapping = Entity::find()
                    .filter(oidc_mapping::Column::OidcSub.eq(oidc_sub))
                    .one(txn)
                    .await?;
                mapping.ok_or_else(|| DbErr::RecordNotFound("Mapping not found after insertion".to_owned()))
            })
        })
        .await
        .flatten_res()
    }
}
