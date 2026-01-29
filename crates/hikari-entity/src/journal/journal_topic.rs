use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "journal_topic")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub journal_summary_id: Uuid,
    pub topic: String,
    pub summary: String,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::journal_summary::Entity",
        from = "Column::JournalSummaryId",
        to = "super::journal_summary::Column::Id"
    )]
    JournalSummary,
}

impl Related<super::journal_summary::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalSummary.def()
    }
}
