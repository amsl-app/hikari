use crate::history;
use crate::util::FlattenTransactionResultExt;
use hikari_entity::history::history_session::{ActiveModel as ActiveHistorySession, Model as HistorySessionModel};
use sea_orm::prelude::*;
use sea_orm::{IntoActiveValue, TransactionTrait};

pub struct Mutation;

impl Mutation {
    pub async fn create<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        module: String,
        session: String,
        conversation_id: Option<Uuid>,
    ) -> Result<HistorySessionModel, DbErr> {
        conn.transaction(|txn| {
            Box::pin(async move {
                let history = history::Mutation::create(txn, user_id).await?;
                let new_history_session = ActiveHistorySession {
                    history_id: history.id.into_active_value(),
                    module: module.into_active_value(),
                    session: session.into_active_value(),
                    conversation_id: conversation_id.into_active_value(),
                };
                new_history_session.insert(txn).await
            })
        })
        .await
        .flatten_res()
    }
}
