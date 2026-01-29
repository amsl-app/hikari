use hikari_db::util::FlattenTransactionResultExt;
use hikari_entity::user::Model as User;
use sea_orm::prelude::*;
use sea_orm::{TransactionError, TransactionTrait};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
enum UserCreationError {
    #[error(transparent)]
    DbErr(#[from] DbErr),

    #[error("User exists")]
    UserExists { user_id: Uuid },
}

pub async fn create_user_id<C: ConnectionTrait + TransactionTrait>(conn: &C, sub: &str) -> Result<Uuid, DbErr> {
    let sub = sub.to_string();

    let res = conn
        .transaction(|txn| {
            Box::pin(async move {
                let user = hikari_db::user::Mutation::create_user(txn).await?;
                let mapping = hikari_db::oidc_mapping::Mutation::create_oidc_mapping(txn, user.id, sub).await?;
                if user.id.eq(&mapping.user_id) {
                    Result::<_, UserCreationError>::Ok(user.id)
                } else {
                    Err(UserCreationError::UserExists {
                        user_id: mapping.user_id,
                    })
                }
            })
        })
        .await;
    let user_id = match res {
        Ok(user_id) => user_id,
        Err(TransactionError::Transaction(UserCreationError::UserExists { user_id })) => user_id,
        Err(TransactionError::Connection(error) | TransactionError::Transaction(UserCreationError::DbErr(error))) => {
            return Err(error);
        }
    };
    Ok(user_id)
}

pub async fn create_user<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
    groups: HashSet<String>,
) -> Result<(User, Vec<String>), DbErr> {
    let sub = sub.to_string();

    conn.transaction(|txn| {
        Box::pin(async move {
            let user_id = create_user_id(txn, sub.as_str()).await?;
            let user = hikari_db::user::Query::find_user_by_id(txn, user_id)
                .await?
                .ok_or_else(|| DbErr::RecordNotFound("Record not found after insertion".to_owned()))?;
            hikari_db::groups::oidc_groups::Mutation::set(txn, user.id, groups).await?;
            let custom_groups = hikari_db::groups::custom_groups::Query::get_for_user(txn, user_id).await?;
            Ok((user, custom_groups))
        })
    })
    .await
    .flatten_res()
}

pub async fn create_user_and_get_token<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
    groups: HashSet<String>,
) -> Result<hikari_entity::access_tokens::Model, DbErr> {
    let sub = sub.to_string();

    conn.transaction(|txn| {
        Box::pin(async move {
            let user_id = create_user_id(txn, &sub).await?;
            let token = hikari_db::access_tokens::Mutation::create_access_token(txn, user_id).await?;

            hikari_db::groups::oidc_groups::Mutation::set(txn, user_id, groups).await?;
            Ok(token)
        })
    })
    .await
    .flatten_res()
}
