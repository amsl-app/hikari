use csml_engine::data::AsyncDatabase;
use hikari_db::history;
use hikari_entity::module::session::status::{Model as InstanceModel, Model, Status};
use sea_orm::prelude::*;
use sea_orm::{ActiveValue, IntoActiveModel, TransactionTrait};

use crate::data::bots::generate_channel_name;

use crate::routes::api::v0::modules::error::ModuleError;
use hikari_db::module::session::status::Mutation;
use hikari_db::util::{FlattenTransactionResultExt, InspectTransactionError};

pub(crate) async fn set_status_as_finished<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    session_status: InstanceModel,
    module_completed: bool,
) -> Result<(), DbErr> {
    if session_status.status != Status::Finished {
        tracing::debug!(user_id = %session_status.user_id, module_id = session_status.module, session_id = session_status.session, "setting session instance to finished");

        let user_id = session_status.user_id;
        conn.transaction(|txn| {
            Box::pin(async move {
                Mutation::set_status_for_user(
                    txn,
                    user_id,
                    &session_status.module,
                    &session_status.session,
                    Status::Finished,
                )
                .await?;
                history::history_session::Mutation::create(
                    txn,
                    user_id,
                    session_status.module.clone(),
                    session_status.session.clone(),
                    session_status.last_conv_id,
                )
                .await?;
                if module_completed {
                    hikari_db::module::status::Mutation::set_status_for_user(
                        txn,
                        user_id,
                        &session_status.module,
                        hikari_entity::module::status::Status::Finished,
                    )
                    .await?;
                    history::history_module::Mutation::create(txn, user_id, session_status.module).await?;
                }
                Result::<_, DbErr>::Ok(())
            })
        })
        .await
        .flatten_res()?;
        tracing::debug!(%user_id, "updated user module entry to finish and set completion date, all completed {module_completed}");
    }
    Ok(())
}

pub(crate) async fn abort_session_instance<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    user_id: Uuid,
    model: Model,
) -> Result<(), ModuleError> {
    tracing::debug!(%user_id, module = %model.module, session = %model.session, "aborting session instance");
    let res = conn
        .transaction(|txn| {
            Box::pin(async move {
                if let (Some(conversation_id), Some(bot_id)) = (model.last_conv_id, &model.bot_id) {
                    csml_engine::future::db_connectors::conversations::close_conversation(
                        conversation_id,
                        &csml_engine::Client {
                            channel_id: generate_channel_name(&model.module, &model.session),
                            user_id: user_id.to_string(),
                            bot_id: bot_id.clone(),
                        },
                        &mut AsyncDatabase::sea_orm(txn),
                    )
                    .await?;
                } else if model.last_conv_id.is_some() {
                    tracing::debug!("No Bot to abort. Maybe websocket session.");
                }
                let mut active_model = model.into_active_model();
                active_model.status = ActiveValue::Set(Status::NotStarted);
                active_model.bot_id = ActiveValue::Set(None);
                active_model.save(txn).await?;
                Result::<(), ModuleError>::Ok(())
            })
        })
        .await;
    res.inspect_transaction_err(
        |error| tracing::error!(error = error as &dyn std::error::Error, %user_id, "error aborting session instance"),
    )
    .flatten_res()
}
