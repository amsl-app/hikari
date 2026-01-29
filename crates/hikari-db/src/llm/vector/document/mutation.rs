use chrono::Utc;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, Set, TransactionTrait};

use hikari_entity::llm::vector::document;
use hikari_entity::llm::vector::document::Entity as DocumentEntity;

pub struct Mutation {}

impl Mutation {
    pub async fn upsert_file<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        id: String,
        hash: Option<String>,
        hash_algorithm: Option<String>,
        name: String,
        link: String,
    ) -> Result<(), DbErr> {
        let file = document::ActiveModel {
            id: Set(id),
            hash: Set(hash),
            hash_algorithm: Set(hash_algorithm),
            created_at: Set(Utc::now().naive_utc()),
            name: Set(name),
            link: Set(link),
        };

        let mut on_conflict = OnConflict::columns([document::Column::Id]);
        on_conflict.update_columns(vec![
            document::Column::Hash,
            document::Column::CreatedAt,
            document::Column::HashAlgorithm,
            document::Column::Name,
            document::Column::Link,
        ]);
        DocumentEntity::insert(file).on_conflict(on_conflict).exec(db).await?;
        Ok(())
    }

    pub async fn remove_file(db: &DatabaseConnection, id: String) -> Result<(), DbErr> {
        let file = hikari_entity::llm::vector::document::ActiveModel {
            id: Set(id.clone()),
            ..Default::default()
        };
        let res = file.delete(db).await?;
        if res.rows_affected == 0 {
            return Err(DbErr::RecordNotFound(id));
        }
        Ok(())
    }
}
