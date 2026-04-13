use hikari_entity::llm::tts::ActiveModel as ActiveVoiceModel;
use hikari_entity::llm::tts::{self, Entity as VoiceEntity};
use sea_orm::ActiveValue::Set;
use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};

pub struct Mutation {}

impl Mutation {
    pub async fn insert_path(db: &DatabaseConnection, message_hash: &str, audio_path: &str) -> Result<(), DbErr> {
        let cache = ActiveVoiceModel {
            message_hash: Set(message_hash.to_string()),
            audio_path: Set(audio_path.to_string()),
        };

        let on_conflict = OnConflict::column(tts::Column::MessageHash).do_nothing().to_owned();

        VoiceEntity::insert(cache).on_conflict(on_conflict).exec(db).await?;

        Ok(())
    }
}
