use chrono::Utc;
use hikari_entity::quiz::quiz;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, Set};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn create_quiz<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: &Uuid,
        module_id: &str,
        session_ids: Vec<String>,
    ) -> Result<quiz::Model, DbErr> {
        let txn = db.begin().await?;

        let quiz = quiz::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(*user_id),
            module_id: Set(module_id.to_string()),
            created_at: Set(Utc::now().naive_utc()),
            status: Set(quiz::Status::Open),
        };

        let quiz_model = quiz.insert(db).await?;

        for session_id in session_ids {
            crate::quiz::quiz_sessions::mutation::Mutation::add_quiz_session(&txn, &quiz_model.id, &session_id).await?;
        }

        txn.commit().await?;

        Ok(quiz_model)
    }

    pub async fn close_quizzes_for_module<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: &Uuid,
        module_id: &str,
    ) -> Result<(), DbErr> {
        let txn = db.begin().await?;

        // Close all open quizzes for the user and module
        let open_quizzes = quiz::Entity::find()
            .filter(quiz::Column::UserId.eq(*user_id))
            .filter(quiz::Column::ModuleId.eq(module_id))
            .filter(quiz::Column::Status.eq(quiz::Status::Open))
            .all(&txn)
            .await?;

        for open_quiz in open_quizzes {
            Self::close_quiz(&txn, &open_quiz.id).await?;
        }

        txn.commit().await?;

        Ok(())
    }

    pub async fn close_quiz<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        quiz_id: &Uuid,
    ) -> Result<quiz::Model, DbErr> {
        let quiz: quiz::ActiveModel = quiz::Entity::find_by_id(*quiz_id)
            .one(db)
            .await?
            .ok_or(DbErr::RecordNotFound(format!("Quiz with id {quiz_id} not found")))?
            .into();

        let quiz = quiz::ActiveModel {
            status: Set(quiz::Status::Closed),
            ..quiz
        };

        quiz.update(db).await
    }
}
