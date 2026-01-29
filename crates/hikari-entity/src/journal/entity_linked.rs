use crate::journal::{journal_entry, journal_entry_tag};
use crate::tag;
use sea_orm::entity::prelude::*;

pub struct JournalEntryToTag;

impl Linked for JournalEntryToTag {
    type FromEntity = journal_entry::Entity;

    type ToEntity = tag::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            journal_entry_tag::Relation::JournalEntry.def().rev(),
            journal_entry_tag::Relation::Tag.def(),
        ]
    }
}
