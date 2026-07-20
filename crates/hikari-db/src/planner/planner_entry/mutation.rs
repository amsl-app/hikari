use std::collections::HashMap;

use chrono::NaiveDate;
use hikari_entity::planner_entry::{ActiveModel, Entity as PlannerEntry, Model as PlannerEntryModel};
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, NotSet, QueryFilter};
use uuid::Uuid;

pub struct PlannerEntryInput {
    pub date: NaiveDate,
    pub title: String,
    pub priority: i32,
    pub module_id: Option<String>,
    pub session_id: Option<String>,
}

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
            updated_at: NotSet,
        };

        let res = entry.insert(db).await;
        res.inspect_err(|error| {
            tracing::error!(error = %error, "failed to create planner entry");
        })
    }

    pub async fn update_planner_entry<C: ConnectionTrait>(
        db: &C,
        mut active_model: ActiveModel,
    ) -> Result<PlannerEntryModel, DbErr> {
        active_model.updated_at = ActiveValue::Set(chrono::Utc::now().naive_utc());
        let res = active_model.update(db).await;
        res.inspect_err(|error| {
            tracing::error!(error = %error, "failed to update planner entry");
        })
    }

    pub async fn create_planner_entries<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        entries: Vec<PlannerEntryInput>,
    ) -> Result<Vec<PlannerEntryModel>, DbErr> {
        if entries.is_empty() {
            return Ok(vec![]);
        }

        let ids: Vec<Uuid> = (0..entries.len()).map(|_| Uuid::new_v4()).collect();
        let order: HashMap<Uuid, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();

        let active_models = ids.iter().zip(entries).map(|(id, input)| ActiveModel {
            id: ActiveValue::Set(*id),
            user_id: ActiveValue::Set(user_id),
            date: ActiveValue::Set(input.date),
            title: ActiveValue::Set(input.title),
            completed: ActiveValue::Set(false),
            priority: ActiveValue::Set(input.priority),
            module_id: ActiveValue::Set(input.module_id),
            session_id: ActiveValue::Set(input.session_id),
            created_at: NotSet,
            updated_at: NotSet,
        });

        PlannerEntry::insert_many(active_models)
            .exec(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = %error, "failed to bulk create planner entries");
            })?;

        let mut models = PlannerEntry::find()
            .filter(hikari_entity::planner_entry::Column::Id.is_in(ids))
            .all(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = %error, "failed to fetch bulk-created planner entries");
            })?;

        models.sort_by_key(|m| order.get(&m.id).copied().unwrap_or(usize::MAX));
        Ok(models)
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
