use hikari_entity::oidc_groups;
use hikari_entity::oidc_groups::{Entity, Model};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_for_user<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(oidc_groups::Column::UserId.eq(user_id))
            .all(db)
            .await
    }
}
