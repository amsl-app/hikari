use chrono::NaiveDate;
use hikari_entity::planner_entry::{Entity as PlannerEntry, Model as PlannerEntryModel};
use hikari_entity::planner_milestone::{Entity as PlannerMilestone, Model as PlannerMilestoneModel};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_user_planner_entries<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<PlannerEntryModel>, DbErr> {
        let mut query = PlannerEntry::find()
            .filter(hikari_entity::planner_entry::Column::UserId.eq(user_id))
            .order_by_desc(hikari_entity::planner_entry::Column::Date);

        if let Some(from) = from {
            query = query.filter(hikari_entity::planner_entry::Column::Date.gte(from));
        }
        if let Some(to) = to {
            query = query.filter(hikari_entity::planner_entry::Column::Date.lte(to));
        }

        let entries = query.all(db).await;

        entries.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load user planner entries");
        })
    }

    /// Same as `get_user_planner_entries`, but also loads each entry's milestone in the same query (LEFT JOIN).
    pub async fn get_user_planner_entries_with_milestone<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<(PlannerEntryModel, Option<PlannerMilestoneModel>)>, DbErr> {
        let mut query = PlannerEntry::find()
            .filter(hikari_entity::planner_entry::Column::UserId.eq(user_id))
            .order_by_desc(hikari_entity::planner_entry::Column::Date);

        if let Some(from) = from {
            query = query.filter(hikari_entity::planner_entry::Column::Date.gte(from));
        }
        if let Some(to) = to {
            query = query.filter(hikari_entity::planner_entry::Column::Date.lte(to));
        }

        let entries = query.find_also_related(PlannerMilestone).all(db).await;

        entries.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load user planner entries with milestone");
        })
    }

    /// Loads a user's planner entry along with its milestone in the same query (LEFT JOIN).
    pub async fn get_user_planner_entry_with_milestone<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Option<(PlannerEntryModel, Option<PlannerMilestoneModel>)>, DbErr> {
        let entry = PlannerEntry::find_by_id(id)
            .filter(hikari_entity::planner_entry::Column::UserId.eq(user_id))
            .find_also_related(PlannerMilestone)
            .one(db)
            .await;

        entry.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load user planner entry with milestone");
        })
    }

    pub async fn get_milestone_entries<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        milestone_id: Uuid,
    ) -> Result<Vec<PlannerEntryModel>, DbErr> {
        let entries = PlannerEntry::find()
            .filter(hikari_entity::planner_entry::Column::UserId.eq(user_id))
            .filter(hikari_entity::planner_entry::Column::MilestoneId.eq(milestone_id))
            .order_by_desc(hikari_entity::planner_entry::Column::Date)
            .all(db)
            .await;

        entries.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load milestone entries");
        })
    }
}
