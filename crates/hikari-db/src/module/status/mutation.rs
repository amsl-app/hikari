use crate::module::status::query;
use chrono::Utc;
use hikari_entity::module::status::{self, Entity as ModuleStatusEntity, Model as ModuleStatus, Status};

use crate::util::FlattenTransactionResultExt;
use sea_orm::prelude::*;
use sea_orm::{ActiveValue, IntoActiveValue, TransactionTrait, TryInsertResult, sea_query};
use uuid::Uuid;

fn create_on_conflict() -> sea_query::OnConflict {
    sea_query::OnConflict::columns([status::Column::UserId, status::Column::Module])
}

pub struct Mutation;

impl Mutation {
    pub async fn create<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        module: String,
    ) -> Result<status::Model, DbErr> {
        let user_module_id = Uuid::new_v4();
        let val = status::ActiveModel {
            module: module.clone().into_active_value(),
            user_id: user_id.into_active_value(),
            status: ActiveValue::Set(Status::NotStarted),
            ..Default::default()
        };

        tracing::trace!(%user_module_id, "inserting user module entry");
        let res = conn
            .transaction(|conn| {
                Box::pin(async move {
                    let mut on_conflict = create_on_conflict();
                    on_conflict.do_nothing();
                    ModuleStatusEntity::insert(val)
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

        let module_status = query::Query::get_for_user(conn, user_id, &module)
            .await?
            .ok_or(DbErr::RecordNotFound(
                "Module status not found after insertion".to_owned(),
            ))?;

        Ok(module_status)
    }

    pub async fn set_status_for_user<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        module: &str,
        status: Status,
    ) -> Result<ModuleStatus, DbErr> {
        let module = module.to_string();
        conn.transaction(|txn| {
            Box::pin(async move {
                let data = status::ActiveModel {
                    module: ActiveValue::Set(module.clone()),
                    user_id: ActiveValue::Set(user_id),
                    status: ActiveValue::Set(status),
                    completion: ActiveValue::NotSet,
                };

                let mut on_conflict = create_on_conflict();
                on_conflict.update_column(status::Column::Status);

                ModuleStatusEntity::insert(data)
                    .on_conflict(on_conflict)
                    .do_nothing()
                    .exec(txn)
                    .await?;

                if status == Status::Finished {
                    let data = status::ActiveModel {
                        completion: ActiveValue::Set(Some(Utc::now().naive_utc())),
                        ..Default::default()
                    };

                    // Update completion separately to not overwrite it if it was already set
                    ModuleStatusEntity::update_many()
                        .set(data)
                        .filter(status::Column::UserId.eq(user_id))
                        .filter(status::Column::Module.eq(&module))
                        .filter(status::Column::Completion.is_null())
                        .exec(txn)
                        .await?;
                }

                query::Query::get_for_user(txn, user_id, &module)
                    .await
                    .and_then(|model| {
                        model.ok_or(DbErr::RecordNotFound(
                            "Module status not found after insertion".to_owned(),
                        ))
                    })
            })
        })
        .await
        .flatten_res()
    }
}
