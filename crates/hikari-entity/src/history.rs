pub mod history_assessment;
pub mod history_module;
pub mod history_session;

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub completed: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(has_one = "history_module::Entity")]
    HistoryModule,
    #[sea_orm(has_one = "history_session::Entity")]
    HistorySession,
    #[sea_orm(has_one = "history_assessment::Entity")]
    HistoryAssessment,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<history_module::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::HistoryModule.def()
    }
}

impl Related<history_session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::HistorySession.def()
    }
}

impl Related<history_assessment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::HistoryAssessment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
