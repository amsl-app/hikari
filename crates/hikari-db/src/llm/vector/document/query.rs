use hikari_entity::llm::vector::document;
use hikari_entity::llm::vector::document::Entity as DocumentEntity;
use hikari_entity::llm::vector::document::Model as DocumentModel;
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait};

pub struct Query;

impl Query {
    pub async fn get_file(db: &DatabaseConnection, file_id: &str) -> Result<Option<DocumentModel>, DbErr> {
        DocumentEntity::find()
            .filter(document::Column::Id.eq(file_id))
            .one(db)
            .await
    }
}
