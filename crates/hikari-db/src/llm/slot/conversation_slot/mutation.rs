use hikari_entity::llm::slot::conversation_slot::{
    ActiveModel as ActiveSlot, Column as SlotColumn, Entity as SlotEntity,
};
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, InsertResult, IntoActiveValue, QueryFilter};
use std::error::Error;
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn insert_or_update_slot(
        db: &DatabaseConnection,
        conversation_id: Uuid,
        slot: String,
        value: String,
    ) -> Result<InsertResult<ActiveSlot>, DbErr> {
        if value.is_empty() {
            tracing::warn!("Slot value is empty");
        }

        let data = ActiveSlot {
            conversation_id: conversation_id.into_active_value(),
            slot: slot.into_active_value(),
            value: value.clone().into_active_value(),
        };

        let mut on_conflict = OnConflict::columns([SlotColumn::ConversationId, SlotColumn::Slot]);

        tracing::debug!(?value, "updating conversation slot with value");
        on_conflict.update_columns(vec![SlotColumn::Value]);

        SlotEntity::insert(data)
            .on_conflict(on_conflict)
            .exec(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "Failed to insert or update slot");
            })
    }

    pub async fn delete_slot_by_name(
        db: &DatabaseConnection,
        conversation_id: Uuid,
        slot: String,
    ) -> Result<(), DbErr> {
        SlotEntity::delete_many()
            .filter(SlotColumn::ConversationId.eq(conversation_id))
            .filter(SlotColumn::Slot.eq(slot))
            .exec(db)
            .await?;

        Ok(())
    }
}
