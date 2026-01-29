use sea_orm::entity::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i16", db_type = "Integer")]
pub enum Status {
    NotStarted = 0,
    Started = 1,
    Finished = 2,
}

impl Status {
    #[must_use]
    pub fn running(&self) -> bool {
        match self {
            Self::Started => true,
            Self::NotStarted | Self::Finished => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "session_status")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub module: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub session: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    pub status: Status,
    pub bot_id: Option<String>,
    pub last_conv_id: Option<Uuid>,
    pub completion: Option<DateTime>,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::user::Entity",
        from = "Column::UserId",
        to = "crate::user::Column::Id"
    )]
    User,
}

impl Related<crate::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}
