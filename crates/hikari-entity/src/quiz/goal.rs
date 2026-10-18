use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "goal")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub module_id: String,
    pub quiz_id: Uuid,
    pub session_ids: Vec<String>,

    pub target_mode: String,
    pub level: String,
    pub deadline: DateTime,
    pub outcome: String,

    #[sea_orm(column_type = "JsonBinary")]
    pub additional_outcomes: Json,

    #[sea_orm(column_type = "JsonBinary")]
    pub obstacles: Json,

    #[sea_orm(column_type = "JsonBinary")]
    pub plans: Json,

    pub active: bool,

    #[sea_orm(column_type = "JsonBinary")]
    pub lecture_targets: Json,
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
