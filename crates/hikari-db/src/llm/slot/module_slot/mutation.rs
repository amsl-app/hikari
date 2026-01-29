use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, InsertResult, IntoActiveValue, QueryFilter};
use std::error::Error;
use uuid::Uuid;

use hikari_entity::llm::slot::module_slot::{
    ActiveModel as ActiveModuleSlot, Column as ModuleSlotColumn, Entity as ModuleSlotEntity,
};
pub struct Mutation;

impl Mutation {
    pub async fn insert_or_update_module_slot(
        db: &DatabaseConnection,
        user_id: Uuid,
        module_id: String,
        slot: String,
        value: String,
    ) -> Result<InsertResult<ActiveModuleSlot>, DbErr> {
        if value.is_empty() {
            tracing::warn!("Slot value is empty");
        }

        let data = ActiveModuleSlot {
            user_id: user_id.into_active_value(),
            module_id: module_id.into_active_value(),
            slot: slot.into_active_value(),
            value: value.clone().into_active_value(),
        };

        let mut on_conflict = OnConflict::columns([
            ModuleSlotColumn::UserId,
            ModuleSlotColumn::ModuleId,
            ModuleSlotColumn::Slot,
        ]);

        tracing::debug!(?value, "updating module slot with value");
        on_conflict.update_columns(vec![ModuleSlotColumn::Value]);

        ModuleSlotEntity::insert(data)
            .on_conflict(on_conflict)
            .exec(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to insert or update module slot");
            })
    }

    pub async fn delete_module_slot_by_name(
        db: &DatabaseConnection,
        user_id: Uuid,
        module_id: String,
        slot: String,
    ) -> Result<(), DbErr> {
        ModuleSlotEntity::delete_many()
            .filter(ModuleSlotColumn::UserId.eq(user_id))
            .filter(ModuleSlotColumn::ModuleId.eq(module_id))
            .filter(ModuleSlotColumn::Slot.eq(slot))
            .exec(db)
            .await?;

        Ok(())
    }

    pub async fn delete_all_module_slots(db: &DatabaseConnection, user_id: Uuid) -> Result<(), DbErr> {
        ModuleSlotEntity::delete_many()
            .filter(ModuleSlotColumn::UserId.eq(user_id))
            .exec(db)
            .await?;

        Ok(())
    }
}
