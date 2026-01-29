use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "history_modules")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub history_id: Uuid,
    pub module: String,
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
