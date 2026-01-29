use futures_util::future::{try_join_all, try_join4};
use hikari_entity::tag;
use hikari_entity::tag::{Entity as Tag, Model as TagModel};

use hikari_entity::journal::{
    journal_content, journal_content::Entity as JournalContent, journal_content::Model as JournalContentModel,
};
use hikari_entity::journal::{
    journal_entry, journal_entry::Entity as JournalEntry, journal_entry::Model as JournalEntryModel,
};

use hikari_entity::journal::{journal_entry_tag, journal_entry_tag::Entity as JournalEntryTag};

use hikari_entity::journal::{journal_prompt::Entity as JournalPrompt, journal_prompt::Model as JournalPromptModel};

use hikari_entity::journal::{
    journal_entry_journal_prompt, journal_entry_journal_prompt::Entity as JournalEntryPrompt,
};
use sea_orm::sea_query::BinOper;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    SelectColumns,
};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_user_journal_entries(
        db: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<Vec<JournalEntryModel>, DbErr> {
        let journal_entries = JournalEntry::find()
            .filter(journal_entry::Column::UserId.eq(user_id))
            .order_by_desc(journal_entry::Column::CreatedAt)
            .all(db)
            .await;

        journal_entries
            .inspect_err(|error| tracing::error!(error = error as &dyn Error, "failed to load user journal entries"))
    }

    /// Find the first `limit` journal entries for the given with non-empty content
    pub async fn get_user_journal_full<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        limit: Option<u64>,
    ) -> Result<
        Vec<(
            JournalEntryModel,
            Vec<JournalContentModel>,
            Vec<TagModel>,
            Vec<JournalPromptModel>,
        )>,
        DbErr,
    > {
        let mut query = JournalEntry::find()
            .select_only()
            .select_column(journal_entry::Column::Id)
            .group_by(journal_entry::Column::Id)
            .filter(journal_entry::Column::UserId.eq(user_id))
            .left_join(JournalContent)
            .column_as(journal_content::Column::Id.count(), "count")
            .order_by_desc(journal_entry::Column::CreatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        let expr = journal_content::Column::Id.count().binary(BinOper::GreaterThan, 0);

        query = query.having(expr);

        tracing::trace!(%user_id, "loading user journal entries");
        let entries: Result<Vec<(Uuid, i64)>, _> = query.into_tuple().all(conn).await;

        let entries = entries
            .inspect_err(|error| tracing::error!(error = error as &dyn Error, "failed to load user journal entries"))?;

        let entries = entries
            .into_iter()
            .map(|(id, _count)| async move {
                let entry = async move {
                    tracing::trace!(%id, "loading journal entry");
                    let entry = JournalEntry::find_by_id(id).one(conn).await?;

                    entry.ok_or_else(|| DbErr::RecordNotFound(format!("Journal entry {id} not found")))
                };

                let contents = async {
                    tracing::trace!(%id, "loading journal content");
                    JournalContent::find()
                        .filter(journal_content::Column::JournalEntryId.eq(id))
                        .order_by_asc(journal_content::Column::CreatedAt)
                        .all(conn)
                        .await
                };

                let focuses = async {
                    tracing::trace!(%id, "loading journal focus");
                    Tag::find()
                        .left_join(JournalEntryTag)
                        .filter(journal_entry_tag::Column::JournalEntryId.eq(id))
                        .filter(tag::Column::Kind.eq(tag::Kind::Focus))
                        .all(conn)
                        .await
                };

                let prompts = async {
                    tracing::trace!(%id, "loading journal prompt");
                    JournalPrompt::find()
                        .left_join(JournalEntryPrompt)
                        .filter(journal_entry_journal_prompt::Column::JournalEntryId.eq(id))
                        .all(conn)
                        .await
                };
                try_join4(entry, contents, focuses, prompts).await.inspect_err(|error| {
                    tracing::error!(error = error as &dyn Error, "failed to load journal entry content");
                })
            })
            .collect::<Vec<_>>();

        try_join_all(entries)
            .await
            .inspect_err(|error| tracing::error!(error = error as &dyn Error, "failed to load user journal entries"))
    }

    pub async fn get_user_journal_entry(
        db: &DatabaseConnection,
        user_id: Uuid,
        journal_entry_id: Uuid,
    ) -> Result<Option<JournalEntryModel>, DbErr> {
        let journal_entry = JournalEntry::find_by_id(journal_entry_id)
            .filter(journal_entry::Column::UserId.eq(user_id))
            .one(db)
            .await;

        journal_entry
            .inspect_err(|error| tracing::error!(error = error as &dyn Error, "failed to load user journal entry"))
    }
}
