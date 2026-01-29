use crate::history;
use crate::util::FlattenTransactionResultExt;
use hikari_entity::history::history_assessment::HistoryAssessmentType;
use hikari_entity::history::history_assessment::{
    ActiveModel as ActiveHistoryAssessment, Model as HistoryAssessmentModel,
};
use sea_orm::prelude::*;
use sea_orm::{ActiveValue, IntoActiveValue, TransactionTrait};

pub struct Mutation;

impl Mutation {
    pub async fn create<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        module: String,
        assessment_type: HistoryAssessmentType,
        assessment_session_id: Uuid,
    ) -> Result<HistoryAssessmentModel, DbErr> {
        conn.transaction(|txn| {
            Box::pin(async move {
                let history = history::Mutation::create(txn, user_id).await?;
                let new_history_session = ActiveHistoryAssessment {
                    history_id: history.id.into_active_value(),
                    module: module.into_active_value(),
                    type_id: ActiveValue::Set(assessment_type),
                    assessment_session_id: assessment_session_id.into_active_value(),
                };
                new_history_session.insert(txn).await
            })
        })
        .await
        .flatten_res()
    }
}
