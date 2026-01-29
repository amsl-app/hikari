use crate::history;
use crate::util::FlattenTransactionResultExt;
use hikari_entity::history::history_module::{ActiveModel as ActiveHistoryModule, Model as HistoryModuleModel};
use sea_orm::prelude::*;
use sea_orm::{IntoActiveValue, TransactionTrait};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn create<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        module: String,
    ) -> Result<HistoryModuleModel, DbErr> {
        conn.transaction(|txn| {
            Box::pin(async move {
                let history = history::Mutation::create(txn, user_id).await?;
                let new_history_module = ActiveHistoryModule {
                    history_id: history.id.into_active_value(),
                    module: module.into_active_value(),
                };
                new_history_module.insert(txn).await
            })
        })
        .await
        .flatten_res()
    }
}
