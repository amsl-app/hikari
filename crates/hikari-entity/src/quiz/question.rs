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

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "question_bloom_level_enum")]
pub enum BloomLevel {
    #[sea_orm(string_value = "remember")]
    Remember,
    #[sea_orm(string_value = "understand")]
    Understand,
    #[sea_orm(string_value = "apply")]
    Apply,
    #[sea_orm(string_value = "analyze")]
    Analyze,
    #[sea_orm(string_value = "evaluate")]
    Evaluate,
    #[sea_orm(string_value = "create")]
    Create,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "question_type_enum")]
pub enum QuestionType {
    #[sea_orm(string_value = "text")]
    Text,
    #[sea_orm(string_value = "multiplechoice")]
    MultipleChoice,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "question")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub quiz_id: Uuid,
    pub session_id: String,
    pub topic: String,
    pub content: String,
    pub question: String,
    pub r#type: QuestionType,
    pub options: Option<String>,
    pub level: BloomLevel,
    pub created_at: DateTime,
    pub answered_at: Option<DateTime>,
    pub answer: Option<String>,
    pub evaluation: Option<String>,
    #[sea_orm(column_type = "Integer")]
    pub grade: Option<i32>,
    pub ai_solution: Option<String>,
    pub status: Status,
    pub feedback: Option<Feedback>,
    pub feedback_explanation: Option<String>,
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
