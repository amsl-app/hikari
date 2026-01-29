use crate::journal::journal_entry_journal_focus;
use crate::util::FlattenTransactionResultExt;
use chrono::{DateTime, FixedOffset};
use futures_util::future::{try_join_all, try_join3};
use hikari_entity::journal::{
    journal_content, journal_entry, journal_entry::Model as JournalEntryModel, journal_entry_journal_prompt,
    journal_prompt, journal_prompt::Model as JournalPromptModel,
};
use hikari_entity::tag::Model as TagModel;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, NotSet, QueryFilter,
    TransactionTrait,
};
use uuid::Uuid;

pub struct Mutation;

pub struct CreatedJournalContent {
    pub id: Uuid,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

pub struct CreatedJournalEntryWithContent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub mood: Option<f32>,
    pub content: Vec<CreatedJournalContent>,
    pub focus: Vec<TagModel>,
    pub prompts: Vec<JournalPromptModel>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

impl Mutation {
    pub async fn create_journal_entry(db: &DatabaseConnection, user_id: Uuid) -> Result<JournalEntryModel, DbErr> {
        let journal_entry = journal_entry::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            user_id: ActiveValue::Set(user_id),
            title: NotSet,
            mood: NotSet,
            created_at: NotSet,
            updated_at: NotSet,
        };

        journal_entry.insert(db).await
    }

    pub async fn set_journal_entry_mood(
        db: &DatabaseConnection,
        user_id: Uuid,
        journal_entry_id: Uuid,
        mood: Option<f32>,
    ) -> Result<(), DbErr> {
        let journal_entry = journal_entry::ActiveModel {
            id: NotSet,
            user_id: ActiveValue::Set(user_id),
            title: NotSet,
            mood: ActiveValue::Set(mood),
            created_at: NotSet,
            updated_at: NotSet,
        };

        let res = journal_entry::Entity::update_many()
            .set(journal_entry)
            .filter(journal_entry::Column::UserId.eq(user_id))
            .filter(journal_entry::Column::Id.eq(journal_entry_id))
            .exec(db)
            .await?;
        if res.rows_affected == 0 {
            return Err(DbErr::RecordNotFound("Journal entry not found".to_string()));
        }
        Ok(())
    }

    pub async fn create_journal_entry_with_content(
        db: &DatabaseConnection,
        user_id: Uuid,
        title: Option<String>,
        contents: Vec<String>,
        focus: Vec<Uuid>,
        mood: Option<f32>,
        prompts: Vec<String>,
    ) -> Result<CreatedJournalEntryWithContent, DbErr> {
        db.transaction::<_, CreatedJournalEntryWithContent, DbErr>(|txn| {
            Box::pin(async move {
                let journal_entry = journal_entry::ActiveModel {
                    id: ActiveValue::Set(Uuid::new_v4()),
                    user_id: ActiveValue::Set(user_id),
                    title: ActiveValue::Set(title),
                    mood: ActiveValue::Set(mood),
                    created_at: NotSet,
                    updated_at: NotSet,
                };

                let journal_entry = journal_entry.insert(txn).await?;

                let content_models = contents
                    .into_iter()
                    .map(|content| async move {
                        let content = journal_content::ActiveModel {
                            id: ActiveValue::Set(Uuid::new_v4()),
                            journal_entry_id: ActiveValue::Set(journal_entry.id),
                            content: ActiveValue::Set(content),
                            title: NotSet,
                            created_at: NotSet,
                            updated_at: NotSet,
                        };
                        content.insert(txn).await
                    })
                    .collect::<Vec<_>>();
                let contents = try_join_all(content_models);

                let prompt_models = prompts
                    .into_iter()
                    .map(|prompt| async move {
                        let active_prompt = journal_prompt::ActiveModel {
                            id: ActiveValue::Set(Uuid::new_v4()),
                            prompt: ActiveValue::Set(prompt.clone()),
                        };
                        let mut on_conflict = OnConflict::column(journal_prompt::Column::Prompt);
                        on_conflict.do_nothing();
                        journal_prompt::Entity::insert(active_prompt)
                            .on_conflict(on_conflict)
                            .do_nothing()
                            .exec(txn)
                            .await?;
                        let prompt = journal_prompt::Entity::find()
                            .filter(journal_prompt::Column::Prompt.eq(&prompt))
                            .one(txn)
                            .await?
                            .ok_or_else(|| {
                                tracing::error!(%prompt, "prompt not found after insertion");
                                DbErr::RecordNotFound("Prompt not found after insertion".to_string())
                            })?;
                        let active_journal_prompt = journal_entry_journal_prompt::ActiveModel {
                            journal_entry_id: ActiveValue::Set(journal_entry.id),
                            journal_prompt_id: ActiveValue::Set(prompt.id),
                        };
                        let mut on_conflict = OnConflict::columns([
                            journal_entry_journal_prompt::Column::JournalEntryId,
                            journal_entry_journal_prompt::Column::JournalPromptId,
                        ]);
                        on_conflict.do_nothing();
                        journal_entry_journal_prompt::Entity::insert(active_journal_prompt)
                            .on_conflict(on_conflict)
                            .do_nothing()
                            .exec(txn)
                            .await?;
                        Ok(prompt)
                    })
                    .collect::<Vec<_>>();
                let prompts = try_join_all(prompt_models.into_iter());

                let focuses = journal_entry_journal_focus::Mutation::set_user_journal_entry_focus(
                    txn,
                    user_id,
                    journal_entry.id,
                    focus,
                );
                let (contents, focus, prompts) = try_join3(contents, focuses, prompts).await?;

                Ok(CreatedJournalEntryWithContent {
                    id: journal_entry.id,
                    user_id,
                    focus,
                    mood: journal_entry.mood,
                    content: contents
                        .into_iter()
                        .map(|model| CreatedJournalContent {
                            id: model.id,
                            created_at: model.created_at,
                            updated_at: model.updated_at,
                        })
                        .collect(),
                    created_at: journal_entry.created_at,
                    updated_at: journal_entry.updated_at,
                    prompts,
                })
            })
        })
        .await
        .flatten_res()
    }
}
