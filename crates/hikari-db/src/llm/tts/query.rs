use hikari_entity::llm::tts::Entity as VoiceEntity;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};
pub struct Query {}

impl Query {
    pub async fn get_path(db: &DatabaseConnection, message_hash: &str) -> Result<Option<String>, DbErr> {
        let entry = VoiceEntity::find_by_id(message_hash).one(db).await?;
        let path = entry.map(|e| e.audio_path);
        Ok(path)
    }
}
