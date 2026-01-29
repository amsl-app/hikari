use hikari_entity::journal::{journal_content, journal_content::Entity as JournalContent};
use sea_orm::{ActiveValue, DatabaseConnection, DbErr, EntityTrait, InsertResult, NotSet};
use std::error::Error;
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn add_journal_content(
        db: &DatabaseConnection,
        journal_entry_id: Uuid,
        content: String,
    ) -> Result<InsertResult<journal_content::ActiveModel>, DbErr> {
        let journal_content = journal_content::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            journal_entry_id: ActiveValue::Set(journal_entry_id),
            content: ActiveValue::Set(content),
            title: NotSet,
            created_at: NotSet,
            updated_at: NotSet,
        };

        let res = JournalContent::insert(journal_content).exec(db).await;

        if let Err(err) = &res {
            tracing::error!(error = err as &dyn Error, "failed to create journal entry content");
        }

        res
    }
}
