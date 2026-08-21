use hikari_entity::quiz::question_recommendation;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, JoinType};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_unused_recommendations_by_quiz(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
    ) -> Result<Vec<question_recommendation::Model>, DbErr> {
        let query = question_recommendation::Entity::find()
            .filter(question_recommendation::Column::QuizId.eq(*quiz_id))
            .filter(question_recommendation::Column::Used.eq(false));

        query.all(db).await.inspect_err(|error| {
            tracing::error!(
                error = error as &dyn Error,
                "failed to load unused recommendations by quiz"
            );
        })
    }
}