use crate::builder::slot::paths::{Destination, SlotPath};

use crate::builder::slot::SlotValuePair;
use crate::execution::error::LlmExecutionError;
use futures_util::future::try_join4;
use hikari_model::llm::message::ConversationMessage;
use hikari_model::llm::slot::Slot;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

pub async fn get_memory(
    conn: &DatabaseConnection,
    conversation_id: &Uuid,
    steps: Option<&[String]>,
    limit: Option<u64>,
) -> Result<Vec<ConversationMessage>, LlmExecutionError> {
    let res: Vec<ConversationMessage> =
        hikari_db::llm::message::Query::get_memory_from_conversation(conn, conversation_id, steps, limit)
            .await?
            .into_iter()
            .map(hikari_model_tools::convert::IntoModel::into_model)
            .collect();
    Ok(res)
}

pub async fn get_slot(
    conn: &DatabaseConnection,
    conversation_id: &Uuid,
    user_id: &Uuid,
    module_id: &str,
    session_id: &str,
    slot: SlotPath,
) -> Result<SlotValuePair, LlmExecutionError> {
    let mut slots = get_slots(
        conn,
        conversation_id,
        user_id,
        module_id,
        session_id,
        vec![slot.clone()],
    )
    .await?;
    if let Some(slot) = slots.pop() {
        Ok(slot)
    } else {
        Err(LlmExecutionError::SlotNotFound(slot))
    }
}

pub async fn get_slots(
    conn: &DatabaseConnection,
    conversation_id: &Uuid,
    user_id: &Uuid,
    module_id: &str,
    session_id: &str,
    slots: Vec<SlotPath>,
) -> Result<Vec<SlotValuePair>, LlmExecutionError> {
    let mut global_names = vec![];
    let mut conv_names = vec![];
    let mut module_names = vec![];
    let mut session_names = vec![];

    for slot in slots {
        let destination = slot.destination();
        match destination {
            Destination::Global => {
                global_names.push(slot.name);
            }
            Destination::Conversation => {
                conv_names.push(slot.name);
            }
            Destination::Module => {
                module_names.push(slot.name);
            }
            Destination::Session => {
                session_names.push(slot.name);
            }
        }
    }

    let (global_slots, conv_slots, module_slots, session_slots) = try_join4(
        get_global_slots(conn, user_id, global_names),
        get_conversation_slots(conn, conversation_id, conv_names),
        get_module_slots(conn, user_id, module_id, module_names),
        get_session_slots(conn, user_id, module_id, session_id, session_names),
    )
    .await?;

    let mut slots: Vec<SlotValuePair> = vec![];

    slots.extend(conv_slots.into_iter().map(|Slot { name, value }| SlotValuePair {
        path: SlotPath::new(name, Destination::Conversation),
        value: value.into(),
    }));

    slots.extend(global_slots.into_iter().map(|Slot { name, value }| SlotValuePair {
        path: SlotPath::new(name, Destination::Global),
        value: value.into(),
    }));

    slots.extend(module_slots.into_iter().map(|Slot { name, value }| SlotValuePair {
        path: SlotPath::new(name, Destination::Module),
        value: value.into(),
    }));

    slots.extend(session_slots.into_iter().map(|Slot { name, value }| SlotValuePair {
        path: SlotPath::new(name, Destination::Session),
        value: value.into(),
    }));

    tracing::trace!(?slots, "Retrieved slot values");

    Ok(slots)
}

pub async fn get_conversation_slots(
    conn: &DatabaseConnection,
    conversation_id: &Uuid,
    slots: Vec<String>,
) -> Result<Vec<Slot>, LlmExecutionError> {
    let slots: Vec<Slot> =
        hikari_db::llm::slot::conversation_slot::Query::get_conversation_slots(conn, conversation_id, Some(slots))
            .await?
            .into_iter()
            .map(hikari_model_tools::convert::IntoModel::into_model)
            .collect();
    Ok(slots)
}

pub async fn get_global_slots(
    conn: &DatabaseConnection,
    user_id: &Uuid,
    slots: Vec<String>,
) -> Result<Vec<Slot>, LlmExecutionError> {
    let slots: Vec<Slot> = hikari_db::llm::slot::global_slot::Query::get_global_slots(conn, user_id, Some(slots))
        .await?
        .into_iter()
        .map(hikari_model_tools::convert::IntoModel::into_model)
        .collect();
    Ok(slots)
}

pub async fn get_session_slots(
    conn: &DatabaseConnection,
    user_id: &Uuid,
    module_id: &str,
    session_id: &str,
    slots: Vec<String>,
) -> Result<Vec<Slot>, LlmExecutionError> {
    let slots: Vec<Slot> =
        hikari_db::llm::slot::session_slot::Query::get_session_slots(conn, user_id, module_id, session_id, Some(slots))
            .await?
            .into_iter()
            .map(hikari_model_tools::convert::IntoModel::into_model)
            .collect();

    Ok(slots)
}

pub async fn get_module_slots(
    conn: &DatabaseConnection,
    user_id: &Uuid,
    module_id: &str,
    slots: Vec<String>,
) -> Result<Vec<Slot>, LlmExecutionError> {
    let slots: Vec<Slot> =
        hikari_db::llm::slot::module_slot::Query::get_module_slots(conn, user_id, module_id, Some(slots))
            .await?
            .into_iter()
            .map(hikari_model_tools::convert::IntoModel::into_model)
            .collect();
    Ok(slots)
}

pub async fn add_usage(
    conn: &DatabaseConnection,
    user_id: &Uuid,
    tokens: u32,
    step: String,
) -> Result<(), LlmExecutionError> {
    tracing::debug!(?tokens, "Tokens used");
    hikari_db::llm::usage::Mutation::add_usage(conn, user_id, tokens, step).await?;
    Ok(())
}
