use std::collections::HashMap;

use crate::planner::planner_entry::query::Query;
use chrono::NaiveDate;
use hikari_entity::planner_entry::{ActiveModel, Entity as PlannerEntry, PlannerEntryWithEffectiveDate};
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, NotSet, QueryFilter};
use uuid::Uuid;

pub struct PlannerEntryInput {
    pub date: NaiveDate,
    pub title: String,
    pub priority: i32,
    pub milestone_id: Option<Uuid>,
}

pub struct Mutation;

impl Mutation {
    /// Re-fetches a single entry with `effective_date` computed, after an insert/update that only
    /// returned the raw row.
    async fn fetch_by_id<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<PlannerEntryWithEffectiveDate, DbErr> {
        Query::get_entry_by_id(db, user_id, id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound(format!("planner entry {id} not found after write")))
    }

    pub async fn create_planner_entry<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        date: NaiveDate,
        title: String,
        priority: i32,
        milestone_id: Option<Uuid>,
    ) -> Result<PlannerEntryWithEffectiveDate, DbErr> {
        let id = Uuid::new_v4();
        let entry = ActiveModel {
            id: ActiveValue::Set(id),
            user_id: ActiveValue::Set(user_id),
            date: ActiveValue::Set(date),
            title: ActiveValue::Set(title),
            completed: ActiveValue::Set(false),
            priority: ActiveValue::Set(priority),
            milestone_id: ActiveValue::Set(milestone_id),
            created_at: NotSet,
            updated_at: NotSet,
        };

        entry.insert(db).await.inspect_err(|error| {
            tracing::error!(error = %error, "failed to create planner entry");
        })?;

        Self::fetch_by_id(db, user_id, id).await
    }

    pub async fn update_planner_entry<C: ConnectionTrait>(
        db: &C,
        mut active_model: ActiveModel,
    ) -> Result<PlannerEntryWithEffectiveDate, DbErr> {
        active_model.updated_at = ActiveValue::Set(chrono::Utc::now().naive_utc());
        let id = *active_model
            .id
            .try_as_ref()
            .expect("id must be set to update a planner entry");
        let user_id = *active_model
            .user_id
            .try_as_ref()
            .expect("user_id must be set to update a planner entry");

        active_model.update(db).await.inspect_err(|error| {
            tracing::error!(error = %error, "failed to update planner entry");
        })?;

        Self::fetch_by_id(db, user_id, id).await
    }

    pub async fn create_planner_entries<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        entries: Vec<PlannerEntryInput>,
    ) -> Result<Vec<PlannerEntryWithEffectiveDate>, DbErr> {
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
            milestone_id: ActiveValue::Set(input.milestone_id),
            created_at: NotSet,
            updated_at: NotSet,
        });

        PlannerEntry::insert_many(active_models)
            .exec(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = %error, "failed to bulk create planner entries");
            })?;

        let mut models = Query::get_entries_by_ids(db, user_id, ids).await?;

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
