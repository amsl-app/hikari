use hikari_entity::quiz::question_recommendation;
use sea_orm::{EntityTrait, ActiveModelTrait, DatabaseConnection, DbErr, Set, TransactionTrait,};
use std::error::Error;
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn create_recommendation(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question_id: &Uuid,
    ) -> Result<question_recommendation::Model, DbErr> {
        let recommendation = question_recommendation::ActiveModel {
            id: Set(Uuid::new_v4()),
            quiz_id: Set(*quiz_id),
            question_id: Set(*question_id),
            recommended_at: Set(chrono::Utc::now().naive_utc()),
            used: Set(false),
            used_at: Set(None),
        };

        recommendation
            .insert(db)
            .await
            .inspect_err(|error| {
                tracing::error!(
                    error = error as &dyn Error,
                    "failed to create question recommendation"
                );
            })
    }

    pub async fn create__multiple_recommendations(
        db: &DatabaseConnection,
        quiz_id: &Uuid,
        question_ids: &[Uuid],
    ) -> Result<Vec<question_recommendation::Model>, DbErr> {
        let txn = db.begin().await?;

        let mut recommendations = Vec::with_capacity(question_ids.len());

        for question_id in question_ids {
            let recommendation = question_recommendation::ActiveModel {
                id: Set(Uuid::new_v4()),
                quiz_id: Set(*quiz_id),
                question_id: Set(*question_id),
                recommended_at: Set(chrono::Utc::now().naive_utc()),
                used: Set(false),
                used_at: Set(None),
            };

            recommendations.push(
                recommendation
                    .insert(&txn)
                    .await
                    .inspect_err(|error| {
                        tracing::error!(
                            error = error as &dyn Error,
                            "failed to create question recommendation"
                        );
                    })?,
            );
        }

        txn.commit().await?;

        Ok(recommendations)
    }

    pub async fn mark_recommendation_as_used(
        db: &DatabaseConnection,
        recommendation_id: &Uuid,
    ) -> Result<question_recommendation::Model, DbErr> {
        let recommendation = question_recommendation::Entity::find_by_id(*recommendation_id)
            .one(db)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound(
                format!("recommendation {} not found", recommendation_id)
            ))?;

        let mut recommendation: question_recommendation::ActiveModel = recommendation.into();

        recommendation.used = Set(true);
        recommendation.used_at = Set(Some(chrono::Utc::now().naive_utc()));

        recommendation.update(db).await
    }
}