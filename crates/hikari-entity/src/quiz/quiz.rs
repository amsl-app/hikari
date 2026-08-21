use sea_orm::entity::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "quiz_status_enum")]
pub enum Status {
    #[sea_orm(string_value = "open")]
    Open,
    #[sea_orm(string_value = "closed")]
    Closed,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "quiz")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub module_id: String,
    pub created_at: DateTime,
    pub status: Status,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::user::Entity",
        from = "Column::UserId",
        to = "crate::user::Column::Id"
    )]
    User,

    #[sea_orm(has_many = "super::quiz_question_attempt::Entity")]
    QuizQuestionAttempt,

    #[sea_orm(has_many = "super::question_recommendation::Entity")]
    QuestionRecommendation,
}

impl Related<crate::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::quiz_question_attempt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::QuizQuestionAttempt.def()
    }
}

impl Related<super::question_recommendation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::QuestionRecommendation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}


