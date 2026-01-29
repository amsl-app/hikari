use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, InsertResult, IntoActiveValue, QueryFilter};
use std::error::Error;
use uuid::Uuid;

use hikari_entity::llm::slot::session_slot::{
    ActiveModel as ActiveSessionSlot, Column as SessionSlotColumn, Entity as SessionSlotEntity,
};
pub struct Mutation;

impl Mutation {
    pub async fn insert_or_update_session_slot(
        db: &DatabaseConnection,
        user_id: Uuid,
        module_id: String,
        session_id: String,
        slot: String,
        value: String,
    ) -> Result<InsertResult<ActiveSessionSlot>, DbErr> {
        if value.is_empty() {
            tracing::warn!("Slot value is empty");
        }

        let data = ActiveSessionSlot {
            user_id: user_id.into_active_value(),
            module_id: module_id.into_active_value(),
            session_id: session_id.into_active_value(),
            slot: slot.into_active_value(),
            value: value.clone().into_active_value(),
        };

        let mut on_conflict = OnConflict::columns([
            SessionSlotColumn::UserId,
            SessionSlotColumn::SessionId,
            SessionSlotColumn::ModuleId,
            SessionSlotColumn::Slot,
        ]);

        tracing::debug!(?value, "updating session slot with value");
        on_conflict.update_columns(vec![SessionSlotColumn::Value]);

        SessionSlotEntity::insert(data)
            .on_conflict(on_conflict)
            .exec(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to insert or update session slot");
            })
    }

    pub async fn delete_session_slot_by_name(
        db: &DatabaseConnection,
        user_id: Uuid,
        module_id: String,
        session_id: String,
        slot: String,
    ) -> Result<(), DbErr> {
        SessionSlotEntity::delete_many()
            .filter(SessionSlotColumn::UserId.eq(user_id))
            .filter(SessionSlotColumn::ModuleId.eq(module_id))
            .filter(SessionSlotColumn::SessionId.eq(session_id))
            .filter(SessionSlotColumn::Slot.eq(slot))
            .exec(db)
            .await?;

        Ok(())
    }

    pub async fn delete_all_session_slots(
        db: &DatabaseConnection,
        user_id: Uuid,
        module_id: String,
    ) -> Result<(), DbErr> {
        SessionSlotEntity::delete_many()
            .filter(SessionSlotColumn::UserId.eq(user_id))
            .filter(SessionSlotColumn::ModuleId.eq(module_id))
            .exec(db)
            .await?;
        Ok(())
    }
}
