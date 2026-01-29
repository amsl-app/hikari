use hikari_entity::oidc_mapping::{Entity, Model};
use sea_orm::{DbConn, DbErr, EntityTrait};

pub struct Query;

impl Query {
    pub async fn find_by_id(db: &DbConn, id: i32) -> Result<Option<Model>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }
}
