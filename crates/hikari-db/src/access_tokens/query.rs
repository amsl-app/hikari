use hikari_entity::access_tokens::{Entity, Model};
use sea_orm::{DbConn, DbErr, EntityTrait};

pub struct Query;

impl Query {
    pub async fn find_by_id(db: &DbConn, id: i32) -> Result<Option<Model>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }
}
