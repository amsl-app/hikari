use chrono::{NaiveDate, NaiveDateTime};
use sea_orm::{FromQueryResult, entity::prelude::*};

/// A planner entry with `effective_date` computed in SQL: unchecked, overdue entries have their
/// effective date pulled forward to today, same as `date` otherwise.
#[derive(Debug, Clone, FromQueryResult)]
pub struct PlannerEntryWithEffectiveDate {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub effective_date: NaiveDate,
    pub title: String,
    pub completed: bool,
    pub priority: i32,
    pub milestone_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "planner_entry")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub title: String,
    pub completed: bool,
    pub priority: i32,
    pub milestone_id: Option<Uuid>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::planner_milestone::Entity",
        from = "Column::MilestoneId",
        to = "super::planner_milestone::Column::Id"
    )]
    Milestone,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::planner_milestone::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Milestone.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
