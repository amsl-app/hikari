use crate::module::session::status::query;
use chrono::Utc;
use hikari_entity::module::session::{
    status, status::Entity as Instance, status::Model as InstanceModel, status::Status,
};

use crate::util::FlattenTransactionResultExt;
use sea_orm::prelude::*;
use sea_orm::{ActiveValue, IntoActiveValue, TransactionTrait, TryInsertResult, sea_query};
use uuid::Uuid;

fn create_on_conflict() -> sea_query::OnConflict {
    sea_query::OnConflict::columns([status::Column::UserId, status::Column::Module, status::Column::Session])
}

pub struct Mutation;

impl Mutation {
    pub async fn create<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        module: String,
        session: String,
        bot: Option<String>,
    ) -> Result<status::Model, DbErr> {
        let user_module_id = Uuid::new_v4();
        let bot_is_some = bot.is_some();
        let val = status::ActiveModel {
            module: module.clone().into_active_value(),
            session: session.clone().into_active_value(),
            user_id: user_id.into_active_value(),
            status: ActiveValue::Set(Status::NotStarted),
            bot_id: bot.into_active_value(),
            ..Default::default()
        };

        tracing::trace!(%user_module_id, "inserting user module entry");
        let res = conn
            .transaction(|conn| {
                Box::pin(async move {
                    let mut on_conflict = create_on_conflict();
                    if bot_is_some {
                        on_conflict.update_column(status::Column::BotId);
                    } else {
                        on_conflict.do_nothing();
                    }
                    Instance::insert(val)
                        .on_conflict(on_conflict)
                        .do_nothing()
                        .exec(conn)
                        .await
                })
            })
            .await
            .flatten_res()?;
        if matches!(res, TryInsertResult::Empty) {
            return Err(DbErr::RecordNotInserted);
        }

        tracing::debug!(%user_module_id, "getting {} user module entry", match res {
            TryInsertResult::Conflicted => "existing",
            _ => "created",
        });

        let user_session_status = query::Query::get_for_user(conn, user_id, &module, &session)
            .await?
            .ok_or(DbErr::RecordNotFound("Record not found after insertion".to_owned()))?;

        Ok(user_session_status)
    }

    pub async fn set_status_for_user<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module: &str,
        session: &str,
        status: Status,
    ) -> Result<InstanceModel, DbErr> {
        let mut data = status::ActiveModel {
            module: ActiveValue::Set(module.to_string()),
            session: ActiveValue::Set(session.to_string()),
            user_id: ActiveValue::Set(user_id),
            status: ActiveValue::Set(status),
            bot_id: ActiveValue::NotSet,
            last_conv_id: ActiveValue::NotSet,
            completion: ActiveValue::NotSet,
        };

        let mut on_conflict = create_on_conflict();
        on_conflict.update_column(status::Column::Status);

        if matches!(status, Status::Finished) {
            data.bot_id = ActiveValue::Set(None);
            on_conflict.update_column(status::Column::BotId);
        }

        Instance::insert(data)
            .on_conflict(on_conflict)
            .do_nothing()
            .exec(conn)
            .await?;

        if matches!(status, Status::Finished) {
            let data = status::ActiveModel {
                completion: ActiveValue::Set(Some(Utc::now().naive_utc())),
                ..Default::default()
            };

            // Update completion separately to not overwrite it if it was already set
            Instance::update_many()
                .set(data)
                .filter(status::Column::UserId.eq(user_id))
                .filter(status::Column::Module.eq(module))
                .filter(status::Column::Session.eq(session))
                .filter(status::Column::Completion.is_null())
                .exec(conn)
                .await?;
        }

        query::Query::get_for_user(conn, user_id, module, session)
            .await
            .and_then(|model| model.ok_or(DbErr::RecordNotFound("Record not found after insertion".to_owned())))
    }
}
