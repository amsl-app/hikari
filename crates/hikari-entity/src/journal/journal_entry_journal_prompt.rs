use sea_orm::entity::prelude::*;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "journal_entry_journal_prompt"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel)]
pub struct Model {
    pub journal_entry_id: Uuid,
    pub journal_prompt_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    JournalEntryId,
    JournalPromptId,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    JournalEntryId,
    JournalPromptId,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = (Uuid, Uuid);

    fn auto_increment() -> bool {
        false
    }
}

impl ColumnTrait for Column {
    type EntityName = Entity;

    fn def(&self) -> ColumnDef {
        match self {
            Self::JournalEntryId | Self::JournalPromptId => ColumnType::Uuid.def(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::journal_entry::Entity",
        from = "Column::JournalEntryId",
        to = "super::journal_entry::Column::Id"
    )]
    JournalEntry,
    #[sea_orm(
        belongs_to = "super::journal_prompt::Entity",
        from = "Column::JournalPromptId",
        to = "super::journal_prompt::Column::Id"
    )]
    JournalPrompt,
}

impl Related<super::journal_prompt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalPrompt.def()
    }
}
