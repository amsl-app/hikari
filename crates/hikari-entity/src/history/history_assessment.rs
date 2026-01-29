use sea_orm::entity::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i16", db_type = "Integer")]
pub enum HistoryAssessmentType {
    Pre = 1,
    Post = 2,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "history_assessment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub history_id: Uuid,
    pub module: String,
    pub type_id: HistoryAssessmentType,
    pub assessment_session_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::history::Entity",
        from = "Column::HistoryId",
        to = "crate::history::Column::Id"
    )]
    History,
}

impl Related<crate::history::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::History.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
