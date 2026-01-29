use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "journal_prompt")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub prompt: String,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::journal_entry_journal_prompt::Entity")]
    JournalEntryPrompt,
}

impl Related<super::journal_entry_journal_prompt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntryPrompt.def()
    }
}

impl Related<super::journal_entry::Entity> for Entity {
    fn to() -> RelationDef {
        super::journal_entry_journal_prompt::Relation::JournalEntry.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::journal_entry_journal_prompt::Relation::JournalPrompt.def().rev())
    }
}
