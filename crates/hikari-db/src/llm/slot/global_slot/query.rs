use hikari_entity::llm::slot::{global_slot, global_slot::Entity as GlobalSlot, global_slot::Model as GlobalSlotModel};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn by_name(
        db: &DatabaseConnection,
        user_id: &Uuid,
        slot: &str,
    ) -> Result<Option<GlobalSlotModel>, DbErr> {
        GlobalSlot::find()
            .filter(global_slot::Column::UserId.eq(*user_id))
            .filter(global_slot::Column::Slot.eq(slot))
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load global slot");
            })
    }

    pub async fn get_global_slots(
        db: &DatabaseConnection,
        user_id: &Uuid,
        slots: Option<Vec<String>>,
    ) -> Result<Vec<GlobalSlotModel>, DbErr> {
        let mut query = GlobalSlot::find().filter(global_slot::Column::UserId.eq(*user_id));
        if let Some(slots) = slots {
            query = query.filter(global_slot::Column::Slot.is_in(slots));
        }
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load global slots from user");
        })
    }
}
