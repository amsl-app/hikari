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

pub async fn get_user_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<(User, Vec<String>), DbErr> {
    let user = hikari_db::user::Query::find_user_by_id(conn, user_id)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("record not found after checking for user id".to_owned()))?;
    let custom_groups = hikari_db::groups::custom_groups::Query::get_for_user(conn, user_id).await?;
    Ok((user, custom_groups))
}

pub async fn check_user_by_sub<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
) -> Result<Option<(User, Vec<String>)>, DbErr> {
    let user_id = hikari_db::oidc_mapping::Query::find_for_sub(conn, sub)
        .await?
        .map(|m| m.user_id);
    if let Some(user_id) = user_id {
        tracing::debug!(%user_id, "UserId found");
        let user = get_user_by_id(conn, user_id).await?;
        return Ok(Some(user));
    }
    Ok(None)
}

pub async fn create_user<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
) -> Result<(User, Vec<String>), DbErr> {
    let _sub = sub.to_string();

    let res = conn
        .transaction(|txn| {
            Box::pin(async move {
                let user = hikari_db::user::Mutation::create_user(txn).await?;
                let mapping = hikari_db::oidc_mapping::Mutation::create_oidc_mapping(txn, user.id, _sub).await?;
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
    let user_id: Uuid = match res {
        Ok(user_id) | Err(TransactionError::Transaction(UserCreationError::UserExists { user_id })) => user_id,
        Err(TransactionError::Connection(error) | TransactionError::Transaction(UserCreationError::DbErr(error))) => {
            return Err(error);
        }
    };
    let user = get_user_by_id(conn, user_id).await?;
    Ok(user)
}

pub async fn get_or_create_user<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
) -> Result<(User, Vec<String>), DbErr> {
    if let Some((user, custom_groups)) = check_user_by_sub(conn, sub).await? {
        tracing::debug!("user already exists, returning existing user");
        Ok((user, custom_groups))
    } else {
        tracing::debug!("user does not exist, create new user");
        let (user, custom_groups) = create_user(conn, sub).await?;
        Ok((user, custom_groups))
    }
}

pub async fn get_or_create_user_and_get_token<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
    groups: HashSet<String>,
) -> Result<hikari_entity::access_tokens::Model, DbErr> {
    let sub = sub.to_string();

    conn.transaction(|txn| {
        Box::pin(async move {
            let (user, _) = get_or_create_user(txn, &sub).await?;
            hikari_db::groups::oidc_groups::Mutation::set(txn, user.id, groups).await?;
            let token = hikari_db::access_tokens::Mutation::create_access_token(txn, user.id).await?;

            Ok(token)
        })
    })
    .await
    .flatten_res()
}
