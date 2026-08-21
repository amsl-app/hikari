use sea_orm::entity::prelude::*;


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
    pub topic: String,
    pub content: String,
    pub question: String,
    pub r#type: QuestionType,
    pub options: Option<String>,
    pub level: BloomLevel,
    pub created_at: DateTime,
    pub ai_solution: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        has_many = "super::quiz_question_attempt::Entity",
    )]
    QuizQuestionAttempt,
}

impl Related<super::quiz_question_attempt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::QuizQuestionAttempt.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
