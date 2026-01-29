use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, InsertResult, IntoActiveValue, QueryFilter};
use std::error::Error;
use uuid::Uuid;

use hikari_entity::llm::slot::global_slot::{
    ActiveModel as ActiveGlobalSlot, Column as GlobalSlotColumn, Entity as GlobalSlotEntity,
};
pub struct Mutation;

impl Mutation {
    pub async fn insert_or_update_global_slot(
        db: &DatabaseConnection,
        user_id: Uuid,
        slot: String,
        value: String,
    ) -> Result<InsertResult<ActiveGlobalSlot>, DbErr> {
        if value.is_empty() {
            tracing::warn!("Slot value is empty");
        }

        let data = ActiveGlobalSlot {
            user_id: user_id.into_active_value(),
            slot: slot.into_active_value(),
            value: value.clone().into_active_value(),
        };

        let mut on_conflict = OnConflict::columns([GlobalSlotColumn::UserId, GlobalSlotColumn::Slot]);

        tracing::debug!(?value, "updating global slot with value");
        on_conflict.update_columns(vec![GlobalSlotColumn::Value]);

        GlobalSlotEntity::insert(data)
            .on_conflict(on_conflict)
            .exec(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to insert or update global slot");
            })
    }

    pub async fn delete_global_slot_by_name(db: &DatabaseConnection, user_id: Uuid, slot: String) -> Result<(), DbErr> {
        GlobalSlotEntity::delete_many()
            .filter(GlobalSlotColumn::UserId.eq(user_id))
            .filter(GlobalSlotColumn::Slot.eq(slot))
            .exec(db)
            .await?;

        Ok(())
    }

    pub async fn delete_all_global_slots(db: &DatabaseConnection, user_id: Uuid) -> Result<(), DbErr> {
        GlobalSlotEntity::delete_many()
            .filter(GlobalSlotColumn::UserId.eq(user_id))
            .exec(db)
            .await?;

        Ok(())
    }
}
