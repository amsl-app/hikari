use chrono::NaiveDate;
use sea_orm::entity::prelude::*;

#[derive(Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Clone, Copy)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "priority_enum")]
pub enum Priority {
    #[sea_orm(string_value = "low")]
    Low,
    #[sea_orm(string_value = "medium")]
    Medium,
    #[sea_orm(string_value = "high")]
    High,
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
    pub priority: Priority,
    pub module_id: Option<String>,
    pub session_id: Option<String>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
