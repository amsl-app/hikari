use hikari_entity::oidc_mapping::{Entity, Model};
use sea_orm::{ColumnTrait, ConnectionTrait, DbConn, DbErr, EntityTrait, QueryFilter, TransactionTrait};

pub struct Query;

impl Query {
    pub async fn find_by_id(db: &DbConn, id: i32) -> Result<Option<Model>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    pub async fn find_for_sub<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        sub: &str,
    ) -> Result<Option<Model>, DbErr> {
        Entity::find()
            .filter(hikari_entity::oidc_mapping::Column::OidcSub.eq(sub))
            .one(db)
            .await
    }
}
