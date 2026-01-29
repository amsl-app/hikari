use sea_orm::entity::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "tag_kind")]
pub enum Kind {
    #[sea_orm(string_value = "focus")]
    Focus,
    #[sea_orm(string_value = "mood")]
    Mood,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tag")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub kind: Kind,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub icon: String,
    pub hidden: bool,
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

    #[sea_orm(has_many = "crate::journal::journal_entry_tag::Entity")]
    Tag,
}

impl Related<crate::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<crate::journal::journal_entry_tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tag.def()
    }
}

impl Related<crate::journal::journal_entry::Entity> for Entity {
    fn to() -> RelationDef {
        crate::journal::journal_entry_tag::Relation::JournalEntry.def()
    }

    fn via() -> Option<RelationDef> {
        Some(crate::journal::journal_entry_tag::Relation::Tag.def().rev())
    }
}
