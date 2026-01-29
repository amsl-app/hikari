use sea_orm::entity::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "module_status_enum")]
pub enum Status {
    #[sea_orm(string_value = "not_started")]
    NotStarted,
    #[sea_orm(string_value = "started")]
    Started,
    #[sea_orm(string_value = "finished")]
    Finished,
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
#[sea_orm(table_name = "module_status")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub module: String,
    pub status: Status,
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
