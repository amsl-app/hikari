use hikari_entity::llm::slot::module_slot::{self, Entity as ModuleSlot, Model as ModuleSlotModel};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn by_name(
        db: &DatabaseConnection,
        user_id: &Uuid,
        module_id: &str,
        slot: &str,
    ) -> Result<Option<ModuleSlotModel>, DbErr> {
        ModuleSlot::find()
            .filter(module_slot::Column::UserId.eq(*user_id))
            .filter(module_slot::Column::ModuleId.eq(module_id))
            .filter(module_slot::Column::Slot.eq(slot))
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load global slot");
            })
    }

    pub async fn get_module_slots(
        db: &DatabaseConnection,
        user_id: &Uuid,
        module_id: &str,
        slots: Option<Vec<String>>,
    ) -> Result<Vec<ModuleSlotModel>, DbErr> {
        let mut query = ModuleSlot::find()
            .filter(module_slot::Column::UserId.eq(*user_id))
            .filter(module_slot::Column::ModuleId.eq(module_id));
        if let Some(slots) = slots {
            query = query.filter(module_slot::Column::Slot.is_in(slots));
        }
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load global slots from user");
        })
    }
}
