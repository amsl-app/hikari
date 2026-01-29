use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "journal_entry")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub mood: Option<f32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
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
    #[sea_orm(has_many = "super::journal_content::Entity")]
    JournalContent,
}

impl Related<crate::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::journal_content::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalContent.def()
    }
}

impl Related<crate::tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::journal_entry_tag::Relation::Tag.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::journal_entry_tag::Relation::JournalEntry.def().rev())
    }
}
