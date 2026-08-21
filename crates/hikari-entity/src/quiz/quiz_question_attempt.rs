
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "question_feedback_enum")]
pub enum Feedback {
    #[sea_orm(string_value = "good")]
    Good,
    #[sea_orm(string_value = "bad")]
    Bad,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "question_status_enum")]
pub enum Status {
    #[sea_orm(string_value = "open")]
    Open,
    #[sea_orm(string_value = "finished")]
    Finished,
    #[sea_orm(string_value = "skipped")]
    Skipped,
}


#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "quiz_question_attempt")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub quiz_id: Uuid,

    #[sea_orm(primary_key)]
    pub question_id: Uuid,

    #[sea_orm(primary_key)]
    pub attempt: i32,

    pub session_id: String,
    pub asked_at: Option<DateTime>,
    pub answered_at: Option<DateTime>,
    pub answer: Option<String>,
    pub evaluation: Option<String>,
    pub grade: Option<i32>,
    pub status: Status,
    pub feedback: Option<Feedback>,
    pub feedback_explanation: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::quiz::Entity",
        from = "Column::QuizId",
        to = "super::quiz::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Quiz,

    #[sea_orm(
        belongs_to = "super::question::Entity",
        from = "Column::QuestionId",
        to = "super::question::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
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