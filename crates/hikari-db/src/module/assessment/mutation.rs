use crate::assessment::answer::QuestionAnswer;
use crate::util::FlattenTransactionResultExt;
use crate::{assessment, history};
use hikari_entity::assessment::session::Model as AssessmentSession;
use hikari_entity::module::assessment::{
    ActiveModel as ActiveModuleAssessment, Column as ModuleAssessmentColumn, Entity as ModuleAssessmentEntity,
    Model as ModuleAssessment,
};
use sea_orm::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, IntoActiveValue, TransactionTrait};
use std::error::Error;
use strum::IntoStaticStr;

pub struct Mutation;

#[derive(Clone, Copy, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum PrePost {
    Pre,
    Post,
}

impl Mutation {
    pub async fn insert_or_update_module_assessment<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module_key: String,
        last_pre: Option<Uuid>,
        last_post: Option<Uuid>,
    ) -> Result<ModuleAssessment, DbErr> {
        let data = ActiveModuleAssessment {
            user_id: user_id.into_active_value(),
            module: module_key.clone().into_active_value(),
            last_pre: last_pre.map_or(ActiveValue::NotSet, |last_pre| ActiveValue::Set(Some(last_pre))),
            last_post: last_post.map_or(ActiveValue::NotSet, |last_post| ActiveValue::Set(Some(last_post))),
        };

        let mut on_conflict = OnConflict::columns([ModuleAssessmentColumn::UserId, ModuleAssessmentColumn::Module]);
        let cols_to_update: Vec<_> = [
            (last_pre, ModuleAssessmentColumn::LastPre),
            (last_post, ModuleAssessmentColumn::LastPost),
        ]
        .into_iter()
        .filter_map(|(last, column)| last.map(|_| column))
        .collect();
        if cols_to_update.is_empty() {
            tracing::debug!("inserting module assessment");
            on_conflict.do_nothing();
        } else {
            tracing::debug!(?cols_to_update, "updating module assessment");
            on_conflict.update_columns(cols_to_update);
        }
        let res = ModuleAssessmentEntity::insert(data)
            .on_conflict(on_conflict)
            .exec_with_returning(conn)
            .await;

        res.inspect_err(|error| {
            tracing::error!(
                error = error as &dyn Error,
                %user_id,
                module_id = module_key,
                "failed to start module assessment session"
            );
        })
    }

    pub async fn start<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        module_key: String,
        assessment: String,
        pre_post: PrePost,
    ) -> Result<(AssessmentSession, ModuleAssessment), DbErr> {
        conn.transaction(|txn| {
            Box::pin(async move {
                let session = assessment::session::Mutation::new_assessment(txn, user_id, assessment).await?;
                let (pre, post) = match pre_post {
                    PrePost::Pre => (Some(session.id), None),
                    PrePost::Post => (None, Some(session.id)),
                };

                let module_assessment =
                    Self::insert_or_update_module_assessment(txn, user_id, module_key, pre, post).await?;
                Ok((session, module_assessment))
            })
        })
        .await
        .flatten_res()
    }

    pub async fn finish<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        module: String,
        pre_post: PrePost,
        answers: Vec<QuestionAnswer>,
        assessment_session_id: Uuid,
    ) -> Result<(), DbErr> {
        conn.transaction(|txn| {
            let module = module.clone();
            Box::pin(async move {
                assessment::session::Mutation::finish_assessment(txn, assessment_session_id, answers).await?;
                history::history_assessment::Mutation::create(
                    txn,
                    user_id,
                    module,
                    match pre_post {
                        PrePost::Pre => hikari_entity::history::history_assessment::HistoryAssessmentType::Pre,
                        PrePost::Post => hikari_entity::history::history_assessment::HistoryAssessmentType::Post,
                    },
                    assessment_session_id,
                )
                .await
            })
        })
        .await
        .flatten_res().inspect_err(
            |error| tracing::warn!(%user_id, module, assessment = Into::<&str>::into(pre_post), error = error as &dyn Error, "failed to save assessment results")
        )?;
        Ok(())
    }
}
