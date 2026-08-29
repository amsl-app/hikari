use crate::util::{FlattenTransactionResultExt, generate_token};
use hikari_entity::{
    access_tokens,
    access_tokens::{ActiveModel, Entity, Model},
};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::*;
use sea_orm::{TransactionTrait, sea_query};

pub struct Mutation;

impl Mutation {
    pub async fn create_access_token<C: TransactionTrait>(conn: &C, user_id: Uuid) -> Result<Model, DbErr> {
        let token = ActiveModel {
            user_id: Set(user_id),
            access_token: Set(generate_token()),
            ..Default::default()
        };

        conn.transaction(|txn| {
            Box::pin(async move {
                // Can't use returning because of how sea_orm works
                Entity::insert(token)
                    .on_conflict(
                        sea_query::OnConflict::column(access_tokens::Column::UserId)
                            .do_nothing()
                            .clone(),
                    )
                    .do_nothing()
                    .exec(txn)
                    .await?;
                let token = Entity::find()
                    .filter(access_tokens::Column::UserId.eq(user_id))
                    .one(txn)
                    .await?;
                token.ok_or(DbErr::RecordNotFound("Token not found after insertion".to_owned()))
            })
        })
        .await
        .flatten_res()
    }

    pub async fn delete_access_token<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<(), DbErr> {
        Entity::delete_many()
            .filter(access_tokens::Column::UserId.eq(user_id))
            .exec(conn)
            .await?;
        Ok(())
    }
}
