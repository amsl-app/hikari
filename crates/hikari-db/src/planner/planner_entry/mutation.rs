use chrono::NaiveDate;
use hikari_entity::planner_entry::{ActiveModel, Entity as PlannerEntry, Model as PlannerEntryModel};
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, NotSet, QueryFilter};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn create_planner_entry<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        date: NaiveDate,
        title: String,
        priority: i32,
        module_id: Option<String>,
        session_id: Option<String>,
    ) -> Result<PlannerEntryModel, DbErr> {
        let entry = ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            user_id: ActiveValue::Set(user_id),
            date: ActiveValue::Set(date),
            title: ActiveValue::Set(title),
            completed: ActiveValue::Set(false),
            priority: ActiveValue::Set(priority),
            module_id: ActiveValue::Set(module_id),
            session_id: ActiveValue::Set(session_id),
            created_at: NotSet,
        };

        let res = entry.insert(db).await;
        res.inspect_err(|error| {
            tracing::error!(error = %error, "failed to create planner entry");
        })
    }

    pub async fn update_planner_entry<C: ConnectionTrait>(
        db: &C,
        active_model: ActiveModel,
    ) -> Result<PlannerEntryModel, DbErr> {
        let res = active_model.update(db).await;
        res.inspect_err(|error| {
            tracing::error!(error = %error, "failed to update planner entry");
        })
    }

    pub async fn delete_planner_entry<C: ConnectionTrait>(db: &C, user_id: Uuid, id: Uuid) -> Result<u64, DbErr> {
        let res = PlannerEntry::delete_many()
            .filter(hikari_entity::planner_entry::Column::Id.eq(id))
            .filter(hikari_entity::planner_entry::Column::UserId.eq(user_id))
            .exec(db)
            .await;

        match res {
            Ok(delete_res) => Ok(delete_res.rows_affected),
            Err(error) => {
                tracing::error!(error = %error, "failed to delete planner entry");
                Err(error)
            }
        }
    }
}
