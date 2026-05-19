use sea_orm::{ConnectionTrait, TransactionTrait, prelude::Uuid};

pub async fn add_usage<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    user_id: &Uuid,
    tokens: u32,
    step: &str,
) -> Result<(), sea_orm::DbErr> {
    tracing::debug!(?tokens, ?step, "tokens used");

    hikari_db::llm::usage::Mutation::add_usage(conn, user_id, tokens, step).await?;
    let step = step.to_string();

    metrics::histogram!(
            "tokens_used",
            "step" => step,
    )
    .record(tokens);

    Ok(())
}
