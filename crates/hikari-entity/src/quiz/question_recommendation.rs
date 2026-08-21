use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "question_recommendation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub quiz_id: Uuid,
    pub question_id: Uuid,
    pub recommended_at: DateTime,
    pub used: bool,
    pub used_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::quiz::Entity",
        from = "Column::QuizId",
        to = "super::quiz::Column::Id"
    )]
    Quiz,

    #[sea_orm(
        belongs_to = "super::question::Entity",
        from = "Column::QuestionId",
        to = "super::question::Column::Id"
    )]
    Question,
}

impl Related<super::quiz::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Quiz.def()
    }
}

impl Related<super::question::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Question.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}