use chrono::NaiveDate;
use hikari_entity::planner_entry::{
    Column as PlannerEntryColumn, Entity as PlannerEntry, PlannerEntryWithEffectiveDate,
};
use hikari_entity::planner_milestone::{Entity as PlannerMilestone, Model as PlannerMilestoneModel};
use sea_orm::sea_query::{CaseStatement, Expr, ExprTrait, SimpleExpr};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

pub struct Query;

/// `effective_date` computed in SQL: unchecked, overdue entries have their effective date pulled
/// forward to `today`, same as `date` otherwise.
fn effective_date_expr(today: NaiveDate) -> SimpleExpr {
    CaseStatement::new()
        .case(
            PlannerEntryColumn::Completed
                .eq(false)
                .and(PlannerEntryColumn::Date.lt(today)),
            Expr::val(today),
        )
        .finally(Expr::col((PlannerEntry, PlannerEntryColumn::Date)))
        .into()
}

impl Query {
    pub async fn get_user_planner_entries<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<PlannerEntryWithEffectiveDate>, DbErr> {
        let today = chrono::Local::now().date_naive();

        let mut query = PlannerEntry::find()
            .filter(PlannerEntryColumn::UserId.eq(user_id))
            .column_as(effective_date_expr(today), "effective_date")
            .order_by_desc(PlannerEntryColumn::Date);

        if let Some(from) = from {
            query = query.filter(effective_date_expr(today).gte(from));
        }
        if let Some(to) = to {
            query = query.filter(effective_date_expr(today).lte(to));
        }

        let entries = query.into_model::<PlannerEntryWithEffectiveDate>().all(db).await;

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
        unchecked: Option<bool>,
    ) -> Result<Vec<(PlannerEntryWithEffectiveDate, Option<PlannerMilestoneModel>)>, DbErr> {
        let today = chrono::Local::now().date_naive();

        let mut query = PlannerEntry::find()
            .filter(PlannerEntryColumn::UserId.eq(user_id))
            .column_as(effective_date_expr(today), "effective_date")
            .order_by_desc(PlannerEntryColumn::Date);

        if let Some(from) = from {
            query = query.filter(effective_date_expr(today).gte(from));
        }
        if let Some(to) = to {
            query = query.filter(effective_date_expr(today).lte(to));
        }
        if unchecked == Some(true) {
            query = query.filter(PlannerEntryColumn::Completed.eq(false));
        }

        let entries = query
            .find_also_related(PlannerMilestone)
            .into_model::<PlannerEntryWithEffectiveDate, PlannerMilestoneModel>()
            .all(db)
            .await;

        entries.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load user planner entries with milestone");
        })
    }

    /// Loads a user's planner entry along with its milestone in the same query (LEFT JOIN).
    pub async fn get_user_planner_entry_with_milestone<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Option<(PlannerEntryWithEffectiveDate, Option<PlannerMilestoneModel>)>, DbErr> {
        let today = chrono::Local::now().date_naive();

        let entry = PlannerEntry::find_by_id(id)
            .filter(PlannerEntryColumn::UserId.eq(user_id))
            .column_as(effective_date_expr(today), "effective_date")
            .find_also_related(PlannerMilestone)
            .into_model::<PlannerEntryWithEffectiveDate, PlannerMilestoneModel>()
            .one(db)
            .await;

        entry.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load user planner entry with milestone");
        })
    }

    /// Loads a user's planner entries by id, with `effective_date` computed in SQL.
    pub async fn get_user_planner_entries_by_ids<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        ids: Vec<Uuid>,
    ) -> Result<Vec<PlannerEntryWithEffectiveDate>, DbErr> {
        let today = chrono::Local::now().date_naive();

        let entries = PlannerEntry::find()
            .filter(PlannerEntryColumn::UserId.eq(user_id))
            .filter(PlannerEntryColumn::Id.is_in(ids))
            .column_as(effective_date_expr(today), "effective_date")
            .into_model::<PlannerEntryWithEffectiveDate>()
            .all(db)
            .await;

        entries.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load user planner entries by ids");
        })
    }

    pub async fn get_milestone_entries<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        milestone_id: Uuid,
    ) -> Result<Vec<PlannerEntryWithEffectiveDate>, DbErr> {
        let today = chrono::Local::now().date_naive();

        let entries = PlannerEntry::find()
            .filter(PlannerEntryColumn::UserId.eq(user_id))
            .filter(PlannerEntryColumn::MilestoneId.eq(milestone_id))
            .column_as(effective_date_expr(today), "effective_date")
            .order_by_desc(PlannerEntryColumn::Date)
            .into_model::<PlannerEntryWithEffectiveDate>()
            .all(db)
            .await;

        entries.inspect_err(|error| {
            tracing::error!(error = %error, "failed to load milestone entries");
        })
    }
}
