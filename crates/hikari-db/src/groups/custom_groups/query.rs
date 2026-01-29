use hikari_entity::custom_groups;
use hikari_entity::custom_groups::{Entity, Model};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_for_user<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<String>, DbErr> {
        let res = Entity::find()
            .filter(custom_groups::Column::UserId.eq(user_id))
            .all(db)
            .await?;
        Ok(res.into_iter().map(|group: Model| group.value).collect())
    }
}
