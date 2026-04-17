use hikari_db::util::FlattenTransactionResultExt;
use hikari_entity::user::Model as User;
use sea_orm::prelude::*;
use sea_orm::{TransactionError, TransactionTrait};
use std::collections::HashSet;

pub async fn check_for_user_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
) -> Result<Option<Uuid>, DbErr> {
    let mapping = hikari_db::oidc_mapping::Query::find_for_sub(conn, sub).await?;
    Ok(mapping.map(|m| m.user_id))
}

pub async fn update_oidc_groups<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    user_id: Uuid,
    groups: &HashSet<String>,
) -> Result<(), DbErr> {
    let current_groups = hikari_db::groups::oidc_groups::Query::get_for_user(conn, user_id).await?;
    let current_group_names: HashSet<String> = current_groups.into_iter().map(|g| g.value).collect();
    if &current_group_names != groups {
        tracing::debug!(
            user_id = user_id.to_string(),
            "Different OIDC groups found, updating groups"
        );
        hikari_db::groups::oidc_groups::Mutation::set(conn, user_id, groups.clone()).await?;
    }
    Ok(())
}

pub async fn get_user<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
    groups: &HashSet<String>,
) -> Result<Option<(User, Vec<String>)>, DbErr> {
    let sub = sub.to_string();
    let user_id = check_for_user_id(conn, &sub).await?;
    if let Some(user_id) = user_id {
        tracing::debug!(user_id = user_id.to_string(), "UserId found");
        update_oidc_groups(conn, user_id, groups).await?;
        let user = hikari_db::user::Query::find_user_by_id(conn, user_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("Record not found after checking for user id".to_owned()))?;
        let custom_groups = hikari_db::groups::custom_groups::Query::get_for_user(conn, user_id).await?;
        tracing::debug!(
            user_id = ?user_id,
            custom_groups = ?custom_groups,
            "Returning user with custom groups"
        );
        return Ok(Some((user, custom_groups)));
    }
    Ok(None)
}

pub async fn create_user<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
    groups: HashSet<String>,
) -> Result<User, DbErr> {
    let sub = sub.to_string();
    let user = conn
        .transaction(|txn| {
            Box::pin(async move {
                let user = hikari_db::user::Mutation::create_user(txn).await?;
                hikari_db::oidc_mapping::Mutation::create_oidc_mapping(txn, user.id, sub).await?;
                hikari_db::groups::oidc_groups::Mutation::set(txn, user.id, groups).await?;
                Ok(user)
            })
        })
        .await;
    let user = match user {
        Ok(user) => user,
        Err(TransactionError::Transaction(err)) | Err(TransactionError::Connection(err)) => return Err(err),
    };
    Ok(user)
}

pub async fn get_or_create_user<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    sub: &str,
    groups: &HashSet<String>,
) -> Result<(User, Vec<String>), DbErr> {
    if let Some((user, custom_groups)) = get_user(conn, sub, groups).await? {
        tracing::debug!("User already exists, returning existing user");
        Ok((user, custom_groups))
    } else {
        tracing::debug!("User does not exist, create new user");
        let user = create_user(conn, sub, groups.clone()).await?;
        Ok((user, Vec::new()))
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
            let (user, _) = get_or_create_user(txn, &sub, &groups).await?;
            let token = hikari_db::access_tokens::Mutation::create_access_token(txn, user.id).await?;

            Ok(token)
        })
    })
    .await
    .flatten_res()
}
