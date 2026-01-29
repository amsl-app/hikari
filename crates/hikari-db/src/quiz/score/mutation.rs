use hikari_entity::quiz::score;
use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, Set};

use uuid::Uuid;
pub struct Mutation;

impl Mutation {
    pub async fn upsert_score(
        db: &DatabaseConnection,
        user_id: &Uuid,
        module_id: &str,
        session_id: &str,
        topic: &str,
        score: &f64,
    ) -> Result<score::Model, DbErr> {
        let on_conflict = OnConflict::columns([
            score::Column::UserId,
            score::Column::ModuleId,
            score::Column::SessionId,
            score::Column::Topic,
        ])
        .update_columns([score::Column::Score])
        .to_owned();

        let score = score::ActiveModel {
            user_id: Set(*user_id),
            module_id: Set(module_id.to_string()),
            session_id: Set(session_id.to_string()),
            topic: Set(topic.to_string()),
            score: Set(*score),
        };
        score::Entity::insert(score)
            .on_conflict(on_conflict)
            .exec_with_returning(db)
            .await
    }
}
