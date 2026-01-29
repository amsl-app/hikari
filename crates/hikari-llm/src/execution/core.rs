use std::time::Duration;

use super::{
    error::LlmExecutionError,
    utils::{get_memory, get_slots},
};
use crate::builder::{
    slot::paths::SlotPath,
    steps::{InjectionTrait, LlmModel, llm::PromptType},
    tools::Tool,
};
use async_openai::types::ChatCompletionRequestMessage;
use hikari_config::module::llm_agent::LlmService;
use hikari_core::openai::{
    CallConfig, Message, OpenAiCallResult, openai_call_with_timeout, streaming::MessageStream, tools::ToolChoice,
};

use hikari_core::llm_config::LlmConfig;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

#[derive(Clone)]
pub struct LlmCore {
    prompt: Vec<PromptType>,
    model: LlmModel,
    memory: Option<Vec<String>>,
    slots: Vec<SlotPath>,
    memory_limit: Option<usize>,
    tool: Option<Tool>,
}

impl LlmCore {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        prompt: Vec<PromptType>,
        model: LlmModel,
        slots: Vec<SlotPath>,
        memory_filter: Option<Vec<String>>,
        memory_limit: Option<usize>,
        tool: Option<Tool>,
    ) -> Self {
        Self {
            prompt,
            model,
            memory: memory_filter,
            slots,
            memory_limit,
            tool,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn invoke(
        &mut self,
        config: &LlmConfig,
        conversation_id: &Uuid,
        user_id: &Uuid,
        module_id: &str,
        session_id: &str,
        llm_service: LlmService,
        conn: DatabaseConnection,
        previous_response: Option<String>,
    ) -> Result<Message, LlmExecutionError> {
        let (prompt, tool) = self
            .inner(conversation_id, user_id, module_id, session_id, conn, previous_response)
            .await?;

        let openai_config = config.get_openai_config(Some(&llm_service));
        let model = self
            .model
            .model
            .as_deref()
            .unwrap_or_else(|| config.get_default_model(Some(&llm_service)));

        let message = openai_call_with_timeout(
            CallConfig::builder()
                .max_retry_interval(Duration::from_secs(1))
                .total_timeout(Duration::from_secs(30))
                .iteration_timeout(Duration::from_secs(5))
                .build(),
            openai_config,
            false,
            self.model.temperature,
            model,
            prompt,
            tool,
            Some(ToolChoice::Required),
        )
        .await?;

        match message {
            OpenAiCallResult::Stream(_) => Err(LlmExecutionError::UnexpectedResponseFormat),
            OpenAiCallResult::Message(msg) => Ok(msg),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn stream(
        &mut self,
        config: &LlmConfig,
        conversation_id: &Uuid,
        user_id: &Uuid,
        module_id: &str,
        session_id: &str,
        llm_service: LlmService,
        conn: DatabaseConnection,
        previous_response: Option<String>,
    ) -> Result<MessageStream, LlmExecutionError> {
        let (prompt, _) = self
            .inner(conversation_id, user_id, module_id, session_id, conn, previous_response)
            .await?;

        let openai_config = config.get_openai_config(Some(&llm_service));
        let model = self
            .model
            .model
            .as_deref()
            .unwrap_or_else(|| config.get_default_model(Some(&llm_service)));

        let answer = openai_call_with_timeout(
            CallConfig::builder()
                .max_retry_interval(Duration::from_secs(1))
                .total_timeout(Duration::from_secs(30))
                .iteration_timeout(Duration::from_secs(5))
                .build(),
            openai_config,
            true,
            self.model.temperature,
            model,
            prompt,
            vec![],
            None,
        )
        .await?;

        match answer {
            OpenAiCallResult::Stream(stream) => Ok(stream),
            OpenAiCallResult::Message(_) => Err(LlmExecutionError::UnexpectedResponseFormat),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn inner(
        &mut self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        module_id: &str,
        session_id: &str,
        conn: DatabaseConnection,
        previous_response: Option<String>,
    ) -> Result<
        (
            Vec<ChatCompletionRequestMessage>,
            Vec<Box<dyn hikari_core::openai::tools::Tool>>,
        ),
        LlmExecutionError,
    > {
        let memory = self.generate_memory(&conn, conversation_id).await?;
        let values = get_slots(
            &conn,
            conversation_id,
            user_id,
            module_id,
            session_id,
            self.slots.clone(),
        )
        .await?;

        let tool = self
            .tool
            .clone()
            .map(|t| t.inject(&values).to_langchain_tool())
            .into_iter()
            .collect::<Vec<_>>();

        let mut formatted_prompt = Vec::with_capacity(self.prompt.len() + memory.len() + 1);

        for prompt in &self.prompt {
            formatted_prompt.push(prompt.inject(&values));
        }

        if let Some(previous_response) = previous_response {
            formatted_prompt.push(PromptType::System(format!("VorherigeAntwort: \n Du hast bereits angefangen eine Antwort zu generieren: '''{previous_response}'''. Generiere eine neue Antwort, die mit der Antwort beginnt, die du bereits generiert hast.").into()));
        }

        formatted_prompt.extend(memory);
        let messages = formatted_prompt
            .into_iter()
            .map(std::convert::TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((messages, tool))
    }

    async fn generate_memory(
        &self,
        conn: &DatabaseConnection,
        conversation_id: &Uuid,
    ) -> Result<Vec<PromptType>, LlmExecutionError> {
        tracing::trace!(steps = ?self.memory, ?conversation_id, "Memory steps");
        let messages = get_memory(
            conn,
            conversation_id,
            self.memory.as_deref(),
            self.memory_limit.map(|l| l as u64),
        )
        .await?;
        tracing::trace!(?messages, "Generated memory");
        Ok(messages.into_iter().map(Into::into).collect::<Vec<_>>())
    }
}
