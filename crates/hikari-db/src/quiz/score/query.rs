use hikari_entity::quiz::score;
use hikari_entity::quiz::score::{Entity as Score, Model as ScoreModel};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;
pub struct Query;

impl Query {
    pub async fn get_scores(db: &DatabaseConnection, user_id: &Uuid) -> Result<Vec<ScoreModel>, DbErr> {
        let query = Score::find().filter(score::Column::UserId.eq(*user_id));
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load scores");
        })
    }

    pub async fn get_scores_by_module(
        db: &DatabaseConnection,
        user_id: &Uuid,
        module_id: &str,
    ) -> Result<Vec<ScoreModel>, DbErr> {
        let query = Score::find()
            .filter(score::Column::ModuleId.eq(module_id))
            .filter(score::Column::UserId.eq(*user_id));
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load scores by module");
        })
    }

    pub async fn get_scores_by_module_session(
        db: &DatabaseConnection,
        user_id: &Uuid,
        module_id: &str,
        session_id: &str,
    ) -> Result<Vec<ScoreModel>, DbErr> {
        let query = Score::find()
            .filter(score::Column::ModuleId.eq(module_id))
            .filter(score::Column::UserId.eq(*user_id))
            .filter(score::Column::SessionId.eq(session_id));
        query.all(db).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, "failed to load scores");
        })
    }

    pub async fn get_score_by_topic(
        db: &DatabaseConnection,
        user_id: &Uuid,
        session_id: &str,
        topic: &str,
    ) -> Result<Option<f64>, DbErr> {
        let query = Score::find()
            .filter(score::Column::UserId.eq(*user_id))
            .filter(score::Column::SessionId.eq(session_id))
            .filter(score::Column::Topic.eq(topic));

        query
            .one(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn std::error::Error, "failed to load score by topic");
            })
            .map(|opt_model| opt_model.map(|m| m.score))
    }
}
