use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "quiz_sessions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub quiz_id: Uuid,
    #[sea_orm(primary_key)]
    pub session_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::quiz::Entity",
        from = "Column::QuizId",
        to = "super::quiz::Column::Id"
    )]
    Quiz,
}

impl Related<super::quiz::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Quiz.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
