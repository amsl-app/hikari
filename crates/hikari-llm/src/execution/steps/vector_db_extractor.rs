use crate::{
    builder::{
        slot::{SaveTarget, paths::SlotPath},
        steps::Condition,
    },
    execution::{error::LlmExecutionError, steps::LlmStepContent, utils::get_slot},
};
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::llm_config::LlmConfig;
use hikari_core::pgvector::search;
use hikari_model::llm::state::{LlmConversationState, LlmStepStatus};
use hikari_utils::values::ValueDecoder;
use sea_orm::DatabaseConnection;
use serde_yml::Value;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{LlmStepResponse, LlmStepTrait};

#[derive(Clone)]
pub struct VectorDBExtractor {
    id: String,
    target: SaveTarget,
    primary_documents: Vec<String>,
    secondary_documents: Vec<String>,
    limit: u32,
    query: SlotPath,
    conditions: Vec<Condition>,
    status: LlmStepStatus,
}

impl VectorDBExtractor {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        target: SaveTarget,
        primary_documents: Vec<String>,
        secondary_documents: Vec<String>,
        limit: u32,
        query: SlotPath,
        conditions: Vec<Condition>,
    ) -> Self {
        Self {
            id,
            target,
            primary_documents,
            secondary_documents,
            limit,
            query,
            conditions,
            status: LlmStepStatus::NotStarted,
        }
    }
}

impl LlmStepTrait for VectorDBExtractor {
    fn call<'a>(
        &'a mut self,
        config: &'a LlmConfig,
        conversation_id: &'a Uuid,
        user_id: &'a Uuid,
        module_id: &'a str,
        session_id: &'a str,
        _llm_service: LlmService,
        conn: DatabaseConnection,
    ) -> BoxFuture<'a, Result<LlmStepResponse, LlmExecutionError>> {
        async move {
            let slot = get_slot(
                &conn,
                conversation_id,
                user_id,
                module_id,
                session_id,
                self.query.clone(),
            )
            .await?;

            let mut context = HashSet::new();

            let queries = match slot.value.as_ref() {
                Value::Sequence(seq) => seq.iter().map(hikari_utils::values::ValueDecoder::encode).collect(),
                other => vec![other.encode()],
            };

            tracing::trace!(?queries, "retriever queries");

            for query in &queries {
                let mut primary_results = Vec::new();
                let mut secondary_results = Vec::new();

                if self.primary_documents.is_empty() && self.secondary_documents.is_empty() {
                    tracing::warn!("No documents provided for vector_db_extractor step");
                    continue;
                } else if self.primary_documents.is_empty() {
                    tracing::warn!("No primary documents provided for vector_db_extractor step");
                    secondary_results = search(
                        config,
                        &conn,
                        &query.clone(),
                        self.limit,
                        self.secondary_documents.as_slice(),
                    )
                    .await?;
                } else if self.secondary_documents.is_empty() {
                    tracing::warn!("No secondary documents provided for vector_db_extractor step");
                    primary_results = search(
                        config,
                        &conn,
                        &query.clone(),
                        self.limit,
                        self.primary_documents.as_slice(),
                    )
                    .await?;
                } else {
                    let limit = self.limit / 2;
                    let remainder = self.limit - (limit * 2);
                    let limit = limit + remainder; // Add remainder to primary to ensure total limit is met

                    primary_results =
                        search(config, &conn, &query.clone(), limit, self.primary_documents.as_slice()).await?;

                    secondary_results = search(
                        config,
                        &conn,
                        &query.clone(),
                        limit,
                        self.secondary_documents.as_slice(),
                    )
                    .await?;
                }
                context.extend(primary_results);
                context.extend(secondary_results);
            }

            tracing::trace!(?context, "retriever context");

            let content: String = context
                .iter()
                .map(|e| format!("**Source: {}**\n{}", e.source.to_string(), e.content))
                .collect::<Vec<_>>()
                .join("\n\n");

            tracing::trace!(content, "formatted content and sources");

            let mut values = HashMap::new();

            if !content.is_empty() {
                values.insert(self.target.clone(), content.into());
            }

            tracing::debug!("Retriever Values: {:?}", values);

            Ok(LlmStepResponse::new(
                LlmStepContent::StepValue {
                    values,
                    next_step: None,
                },
                None,
            ))
        }
        .boxed()
    }

    fn add_previous_response(&mut self, _response: String) {
        tracing::error!(
            "Adding previous response to vector_db_extractor should not happen, since this step does not produce a response."
        );
    }

    fn remove_previous_response(&mut self) {
        // Nothing will happen here; Function gets called at the beginning of the step
    }

    fn set_status(&mut self, status: LlmStepStatus) -> LlmConversationState {
        self.status = status;
        self.state()
    }

    fn finish(&mut self) -> LlmConversationState {
        self.set_status(LlmStepStatus::Completed);
        self.state()
    }

    fn status(&self) -> LlmStepStatus {
        self.status
    }

    fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    fn id(&self) -> &str {
        &self.id
    }
}
