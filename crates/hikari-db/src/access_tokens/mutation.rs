use crate::util::FlattenTransactionResultExt;
use base64::Engine;
use hikari_entity::{
    access_tokens,
    access_tokens::{ActiveModel, Entity, Model},
};
use ring::rand::{self, SecureRandom};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::*;
use sea_orm::{TransactionTrait, sea_query};

pub struct Mutation;

fn generate_token() -> String {
    let rng = rand::SystemRandom::new();
    // TODO (LOW): once MaybeUnit::uninit_array is stabilized, use it here
    let mut bytes = [0u8; 64];
    // This should never fail because the only function that can fail here is getentropy
    // which should never fail on a modern system.
    // (The length error is impossible because the length is shorter than 256 bytes
    //    and the library would also handle longer lengths correctly)
    rng.fill(&mut bytes).expect("Failed to generate random bytes");
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let token = generate_token();
        let token = base64::engine::general_purpose::STANDARD.decode(&token).unwrap();
        assert_eq!(token.len(), 64);
        // If this does happen we probably forgot to fill the buffer with random bytes
        token
            .iter()
            .find(|&&b| b != 0)
            .expect("token is all zeros, this should never happen");
    }
}
