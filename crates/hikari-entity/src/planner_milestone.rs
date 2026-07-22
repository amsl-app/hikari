use chrono::NaiveDate;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "planner_milestone")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub date: NaiveDate,
    pub description: Option<String>,
    pub module_id: Option<String>,
    pub origin_id: Option<String>,
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
    #[sea_orm(has_many = "super::planner_entry::Entity")]
    PlannerEntry,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::planner_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PlannerEntry.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
