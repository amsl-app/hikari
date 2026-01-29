use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveValue, QueryFilter,
    TransactionTrait,
};
use uuid::Uuid;

use crate::util::FlattenTransactionResultExt;
use base64::Engine;
use chrono::NaiveDateTime;
use hikari_entity::journal::journal_summary::{ActiveModel, Model};
use hikari_entity::journal::journal_topic;
use hikari_entity::journal::journal_topic::{
    ActiveModel as ActiveJournalTopic, Entity as JournalTopicEntity, Model as JournalTopicModel,
};

pub struct Topic {
    pub title: String,
    pub summary: String,
}

pub struct Mutation;

impl Mutation {
    pub async fn create<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        timestamp: NaiveDateTime,
        key: &[u8; 32],
        summary: String,
        topics: Vec<Topic>,
    ) -> Result<(Model, Vec<JournalTopicModel>), DbErr> {
        let summary_id = Uuid::new_v4();
        let summary = ActiveModel {
            id: summary_id.into_active_value(),
            user_id: user_id.into_active_value(),
            key: key.to_vec().into_active_value(),
            created_at: ActiveValue::Set(timestamp),
            summary: summary.into_active_value(),
        };

        let hex_key = base64::engine::general_purpose::STANDARD.encode(key);
        tracing::debug!(%user_id, key = hex_key, "saving user journal summary");

        let (summary, topics) = conn
            .transaction(|txn| {
                Box::pin(async move {
                    let summary = summary.insert(txn).await?;
                    let topics = if topics.is_empty() {
                        vec![]
                    } else {
                        let topics = topics.into_iter().map(|topic| ActiveJournalTopic {
                            id: Uuid::new_v4().into_active_value(),
                            journal_summary_id: summary_id.into_active_value(),
                            topic: topic.title.into_active_value(),
                            summary: topic.summary.into_active_value(),
                        });
                        JournalTopicEntity::insert_many(topics).exec(txn).await?;
                        JournalTopicEntity::find()
                            .filter(journal_topic::Column::JournalSummaryId.eq(summary_id))
                            .all(txn)
                            .await?
                    };
                    Result::<_, DbErr>::Ok((summary, topics))
                })
            })
            .await
            .flatten_res()?;

        Ok((summary, topics))
    }
}
