use crate::convert::FromDbModel;
use hikari_entity::journal::journal_entry::Model as JournalEntryModel;
use hikari_model::journal::JournalEntry;

impl FromDbModel<JournalEntryModel> for JournalEntry {
    fn from_db_model(model: JournalEntryModel) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            title: model.title,
            mood: model.mood,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
