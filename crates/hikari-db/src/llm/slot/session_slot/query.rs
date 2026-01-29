use hikari_entity::llm::slot::session_slot::{self, Entity as SessionSlot, Model as SessionSlotModel};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn by_name(
        db: &DatabaseConnection,
        user_id: &Uuid,
        module_id: &str,
        session_id: &str,
        slot: &str,
    ) -> Result<Option<SessionSlotModel>, DbErr> {
        SessionSlot::find()
            .filter(session_slot::Column::UserId.eq(*user_id))
            .filter(session_slot::Column::ModuleId.eq(module_id))
            .filter(session_slot::Column::SessionId.eq(session_id))
            .filter(session_slot::Column::Slot.eq(slot))
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn Error, "failed to load global slot");
            })
    }

    pub async fn get_session_slots(
        db: &DatabaseConnection,
        user_id: &Uuid,
        module_id: &str,
        session_id: &str,
        slots: Option<Vec<String>>,
    ) -> Result<Vec<SessionSlotModel>, DbErr> {
        let mut query = SessionSlot::find()
            .filter(session_slot::Column::UserId.eq(*user_id))
            .filter(session_slot::Column::ModuleId.eq(module_id))
            .filter(session_slot::Column::SessionId.eq(session_id));
        if let Some(slots) = slots {
            query = query.filter(session_slot::Column::Slot.is_in(slots));
        }
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load global slots from user");
        })
    }
}
