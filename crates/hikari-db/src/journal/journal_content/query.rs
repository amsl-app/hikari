use hikari_entity::journal::journal_entry;
use hikari_entity::journal::{
    journal_content, journal_content::Entity as JournalContent, journal_content::Model as JournalContentModel,
};
use sea_orm::JoinType::LeftJoin;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, RelationTrait};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_user_journal_entry_contents(
        db: &DatabaseConnection,
        user_id: Uuid,
        journal_entry_id: Uuid,
    ) -> Result<Vec<JournalContentModel>, DbErr> {
        let res = JournalContent::find()
            .join(LeftJoin, journal_content::Relation::JournalEntry.def())
            .filter(journal_content::Column::JournalEntryId.eq(journal_entry_id))
            .filter(journal_entry::Column::UserId.eq(user_id))
            .all(db)
            .await;

        if let Err(error) = &res {
            tracing::error!(error = error as &dyn Error, "failed to load journal entry contents");
        }

        res
    }

    pub async fn get_user_journal_content(
        db: &DatabaseConnection,
        user_id: Uuid,
        // journal_entry_id: Uuid,
        content_id: Uuid,
    ) -> Result<Option<JournalContentModel>, DbErr> {
        // As the content id is globally unique, we don't need the journal_entry_id, so we don't use it
        let res = JournalContent::find_by_id(content_id)
            .join(LeftJoin, journal_content::Relation::JournalEntry.def())
            // .filter(journal_content::Column::JournalEntryId.eq(journal_entry_id))
            .filter(journal_entry::Column::UserId.eq(user_id))
            .one(db)
            .await;

        if let Err(error) = &res {
            tracing::error!(error = error as &dyn Error, "failed to load journal entry content");
        }

        res
    }
}
