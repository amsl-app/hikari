use crate::data::csml::{message_from_csml_message, message_from_csml_message_data};
use crate::db;
use crate::routes::api::v0::bots::{MetaUser, Metadata, ModuleMetadata};
use crate::routes::api::v0::modules::error::MessagingError;
use crate::routes::api::v0::modules::messaging::generate_client;
use chrono::Utc;
use csml_engine::data::AsyncDatabase;
use csml_engine::data::models::BotOpt;
use csml_interpreter::data::CsmlBot;
use hikari_common::csml_utils::init_request;
use hikari_config::module::session::Session;
use hikari_config::module::{Module, ModuleCategory, ModuleConfig};
use hikari_db::module::session::status;
use hikari_entity::module::session::status::{Entity as SessionEntity, Model as SessionModel};
use hikari_llm::execution::error::LlmExecutionError;
use hikari_model::chat::{CreatableEntity, MessageResponse, Payload, RequestMetadata};
use hikari_model::llm::conversation::LlmConversation;
use hikari_model::llm::message::ConversationMessage;
use hikari_model::llm::state::LlmConversationState;
use hikari_model::user::User;
use hikari_model_tools::convert::IntoModel;
use sea_orm::{ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel, TransactionTrait};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::iter;
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn start_conversation<C: ConnectionTrait + TransactionTrait + Send>(
    conn: &C,
    user: User,
    payload: Payload,
    metadata: RequestMetadata,
    csml_bot: CsmlBot,
    module: &Module<'_>,
    session: &Session,
    session_entry: SessionModel,
    configs: Map<String, Value>,
    module_config: &ModuleConfig,
) -> Result<MessageResponse<Payload>, MessagingError> {
    tracing::debug!(%user.id, "starting conversation");
    let module_completion = hikari_db::module::status::Query::all(conn, user.id)
        .await?
        .into_iter()
        .map(|m| (m.module, m.completion))
        .collect::<HashMap<_, _>>();
    let module_metadata = module_config
        .modules()
        .values()
        .map(|module| {
            let completion = module_completion.get(&module.id).copied().unwrap_or(None);
            ModuleMetadata {
                id: module.id.clone(),
                category: module.category,
                completion,
            }
        })
        .collect::<Vec<_>>();

    let metadata = Metadata {
        user: MetaUser {
            name: user.name,
            birthday: user.birthday,
            subject: user.subject,
            semester: user.semester,
            gender: user.gender.map(|g| g.to_string()),
            groups: user.groups,
            configs: Value::Object(configs),
        },
        time: metadata.time.unwrap_or_else(|| Utc::now().fixed_offset()),
        modules: module_metadata,
    };
    // let (http, conv) = no_db_start_conversation(user, metadata, (payload, csml_bot), &session_entry)?;

    if !session_entry.status.running() {
        return Err(MessagingError::NotRunning);
    }
    let client = generate_client(
        user.id,
        csml_bot.id.clone(),
        &session_entry.module,
        &session_entry.session,
    );

    let metadata = serde_json::to_value(metadata)?;
    tracing::debug!(%user.id, ?metadata, "message metadata for user");

    let payload = serde_json::to_value(payload)?;

    let request = init_request(client.clone(), payload, metadata);
    let request_id = request.request_id.clone();

    let bot_opt = BotOpt::CsmlBot(Box::new(csml_bot));

    let mut db = AsyncDatabase::sea_orm(conn);

    let (conversation, messages) = csml_engine::future::start_conversation_db(request, bot_opt, &mut db)
        .await
        .inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "start conversation failed");
        })?;

    let conversation_id = conversation.id;
    let session_entry = if session_entry
        .last_conv_id
        .as_ref()
        .is_none_or(|last| last != &conversation_id)
    {
        tracing::debug!(user_id = %user.id, conversation_id = %conversation_id,
            "updated last conversation id"
        );
        let mut active_model = session_entry.into_active_model();
        active_model.last_conv_id = ActiveValue::Set(Some(conversation_id));

        SessionEntity::update(active_model).exec(conn).await?
    } else {
        session_entry
    };

    let mut created_entities: Vec<CreatableEntity> = vec![];
    if conversation.is_closed() {
        let sessions = status::Query::get_finished_sessions(conn, user.id, &session_entry.module).await?;

        let sessions_set: HashSet<_> = sessions
            .iter()
            .map(|model| &model.session)
            .chain(iter::once(&session_entry.session))
            .collect();
        let module_finished = module
            .sessions
            .values()
            .all(|session| sessions_set.contains(&session.id));

        super::module::session::user_session_status::set_status_as_finished(conn, session_entry, module_finished)
            .await?;
        if module.category == ModuleCategory::Journal {
            let journal_entry =
                db::sea_orm::journal::create_session_journal_entry(conn, session.title.as_str(), conversation.id)
                    .await?;
            if let Some(journal_entry) = journal_entry {
                created_entities.push(CreatableEntity::JournalEntry(journal_entry));
            }
        }
    }

    let messages = messages
        .into_iter()
        .map(|message_data| {
            if message_data.payload.content_type == "error" {
                let user_id = user.id;
                tracing::error!(name: "error_in_csml_output", %user_id, %conversation_id, module = %module.id, session = %session.id, error = ?message_data.payload, "error in csml output");
            }
            message_from_csml_message_data(message_data)
        })
        .collect();

    Ok(MessageResponse {
        client: Some(client),
        request_id: Some(request_id),
        conversation_id: Some(conversation.id),
        conversation_end: conversation.is_closed(),
        history: false,
        messages,
        created_entities,
    })
}

pub(crate) async fn start_new_llm_conversation<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    user_id: Uuid,
    module_id: &str,
    session_id: &str,
) -> Result<LlmConversation, LlmExecutionError> {
    hikari_db::llm::conversation::Mutation::close_open_conversations(conn, user_id, module_id, session_id).await?;
    let model: LlmConversation = hikari_db::llm::conversation::Mutation::create_conversation(
        conn,
        user_id,
        module_id.to_owned(),
        session_id.to_owned(),
    )
    .await?
    .into_model();

    // Set module status to started if it is not already started
    let module_status = hikari_db::module::status::Mutation::create(conn, user_id, module_id.to_owned()).await?;
    if module_status.status != hikari_entity::module::status::Status::Started {
        hikari_db::module::status::Mutation::set_status_for_user(
            conn,
            user_id,
            module_id,
            hikari_entity::module::status::Status::Started,
        )
        .await?;
    }

    // Set session status to started if it is not already started
    let session_status =
        status::Mutation::create(conn, user_id, module_id.to_owned(), session_id.to_owned(), None).await?;

    if session_status.status != hikari_entity::module::session::status::Status::Started {
        status::Mutation::set_status_for_user(
            conn,
            user_id,
            module_id,
            session_id,
            hikari_entity::module::session::status::Status::Started,
        )
        .await?;
    }
    let mut active_model = session_status.into_active_model();
    active_model.last_conv_id = ActiveValue::Set(Some(model.conversation_id));
    SessionEntity::update(active_model).exec(conn).await?;

    Ok(model)
}

pub(crate) async fn get_last_or_create_llm_conversation<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    user_id: Uuid,
    module_id: &str,
    session_id: &str,
) -> Result<(LlmConversation, Option<LlmConversationState>), LlmExecutionError> {
    tracing::trace!(
        ?user_id,
        ?module_id,
        ?session_id,
        "Trying to get last conversation by module, session and user"
    );
    let model = hikari_db::llm::conversation::Query::get_last_conversation_by_module_session_user(
        conn, user_id, module_id, session_id,
    )
    .await?;
    let (model, state) = if let Some(model) = model {
        let state =
            hikari_db::llm::conversation_state::Query::get_conversation_state(conn, model.conversation_id).await?;
        let state: Option<LlmConversationState> = state.map(hikari_model_tools::convert::IntoModel::into_model);
        let state = match state {
            Some(mut state) => {
                let not_finished_message = hikari_db::llm::message::Query::get_not_finished_message(
                    conn,
                    model.conversation_id,
                    &state.current_step,
                )
                .await?;
                if let Some(not_finished_message) = not_finished_message {
                    let message: ConversationMessage = not_finished_message.into_model();
                    state.value.response = message.message.message_string();
                }
                Some(state)
            }
            None => None,
        };
        (model.into_model(), state)
    } else {
        tracing::trace!(
            ?user_id,
            ?module_id,
            ?session_id,
            "Starting new conversation for user, module and session"
        );
        let model = start_new_llm_conversation(conn, user_id, module_id, session_id).await?;
        (model, None)
    };
    Ok((model, state))
}

pub(crate) async fn finish_llm_conversation<'a, C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    user_id: Uuid,
    module: &'a Module<'a>,
    session_id: &str,
) -> Result<(), LlmExecutionError> {
    let session_entry = status::Mutation::create(conn, user_id, module.id.clone(), session_id.to_owned(), None).await?;
    let sessions = status::Query::get_finished_sessions(conn, user_id, &module.id).await?;
    let sessions_set: HashSet<&str> = sessions
        .iter()
        .map(|m| m.session.as_str())
        .chain(iter::once(session_id))
        .collect();

    let module_finished = module
        .sessions
        .values()
        .all(|session| sessions_set.contains(session.id.as_str()));

    super::module::session::user_session_status::set_status_as_finished(conn, session_entry, module_finished).await?;
    Ok(())
}

pub(crate) async fn get_conversation_history<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    conversation_id: Uuid,
) -> Result<MessageResponse<Payload>, MessagingError> {
    tracing::debug!(%conversation_id, "loading conversation history");

    let mut db = AsyncDatabase::sea_orm(conn);
    let conversation =
        csml_engine::future::db_connectors::conversations::get_conversation(&mut db, conversation_id).await?;

    let messages =
        csml_engine::future::db_connectors::messages::get_conversation_messages(&mut db, conversation_id).await?;

    let conversation_end = conversation.is_closed();

    Ok(MessageResponse {
        client: Some(conversation.client),
        request_id: None,
        conversation_id: Some(conversation_id),
        conversation_end,
        history: true,
        messages: messages
            .into_iter()
            .rev() // Messages are returned in reverse order
            .map(message_from_csml_message)
            .collect(),
        created_entities: vec![],
    })
}
