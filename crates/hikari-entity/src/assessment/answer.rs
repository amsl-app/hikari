use sea_orm::entity::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i16", db_type = "Integer")]
pub enum AnswerType {
    Int = 1,
    Text = 2,
    Bool = 3,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "answer")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub assessment_session_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub question: String,
    pub answer_type: AnswerType,
    pub data: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::session::Entity",
        from = "Column::AssessmentSessionId",
        to = "super::session::Column::Id"
    )]
    Session,
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
