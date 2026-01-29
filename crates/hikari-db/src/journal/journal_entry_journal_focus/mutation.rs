use hikari_entity::journal::{journal_entry, journal_entry_tag};
use hikari_entity::tag;
use hikari_entity::tag::{Entity as Tag, Model as TagModel};

use crate::util::FlattenTransactionResultExt;
use sea_orm::{
    ActiveValue, ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter, TransactionTrait,
};
use std::error::Error;
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn set_user_journal_entry_focus<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: Uuid,
        journal_entry_id: Uuid,
        focus: Vec<Uuid>,
    ) -> Result<Vec<TagModel>, DbErr> {
        let count = focus.len();

        let focus = Tag::find()
            .filter(
                Condition::any()
                    .add(tag::Column::UserId.eq(user_id))
                    .add(tag::Column::UserId.is_null()),
            )
            .filter(tag::Column::Id.is_in(focus))
            .filter(tag::Column::Kind.eq(tag::Kind::Focus))
            .all(db)
            .await?;

        if count != focus.len() {
            tracing::error!(user_id = %user_id, journal_entry_id = %journal_entry_id, "did not find the correct number of focus entities");
            return Err(DbErr::RecordNotFound("focus".to_string()));
        }

        let journal_entry = journal_entry::Entity::find_by_id(journal_entry_id)
            .filter(journal_entry::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("failed to find journal entry".to_string()))?;

        let res = db.transaction::<_, Vec<TagModel>, DbErr>(|txn| {
            Box::pin(async move {
                tracing::debug!(user_id = %user_id, journal_entry_id = %journal_entry_id, "deleting existing focus");
                journal_entry_tag::Entity::delete_many().filter(
                    journal_entry_tag::Column::JournalEntryId.eq(journal_entry.id)
                ).exec(txn).await?;
                if !focus.is_empty() {
                    tracing::debug!(user_id = %user_id, journal_entry_id = %journal_entry_id, "setting new focus");
                    journal_entry_tag::Entity::insert_many(
                        focus.iter().map(|model| journal_entry_tag::ActiveModel {
                            journal_entry_id: ActiveValue::Set(journal_entry.id),
                            tag_id: ActiveValue::Set(model.id),
                        }).collect::<Vec<_>>()
                    ).exec(txn).await?;
                }
                Ok(focus)
            })
        }).await.flatten_res();

        match res {
            Ok(focus) => Ok(focus),
            Err(err) => {
                tracing::error!(error = &err as &dyn Error, "failed to set focus");
                Err(err)
            }
        }
    }
}
