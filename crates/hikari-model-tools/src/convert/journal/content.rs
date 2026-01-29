use crate::convert::FromDbModel;
use hikari_entity::journal::journal_content::Model as JournalContentModel;
use hikari_model::journal::content::JournalContent;

impl FromDbModel<JournalContentModel> for JournalContent {
    fn from_db_model(model: JournalContentModel) -> Self {
        Self {
            id: model.id,
            journal_entry_id: model.journal_entry_id,
            title: model.title,
            content: model.content,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
