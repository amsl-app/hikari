use hikari_entity::llm::slot::conversation_slot as slot;
use hikari_entity::llm::slot::conversation_slot::{Entity as Slot, Model as SlotModel};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;
pub struct Query;

impl Query {
    pub async fn get_conversation_slots(
        db: &DatabaseConnection,
        conversation_id: &Uuid,
        slots: Option<Vec<String>>,
    ) -> Result<Vec<SlotModel>, DbErr> {
        let mut query = Slot::find().filter(slot::Column::ConversationId.eq(*conversation_id));
        if let Some(slots) = slots {
            query = query.filter(slot::Column::Slot.is_in(slots));
        }
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load slots");
        })
    }

    pub async fn by_name(
        db: &DatabaseConnection,
        conversation_id: &Uuid,
        slot: String,
    ) -> Result<Option<SlotModel>, DbErr> {
        Slot::find()
            .filter(slot::Column::ConversationId.eq(*conversation_id))
            .filter(slot::Column::Slot.eq(slot))
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load slot");
            })
    }
}
