use hikari_entity::user_context_logs::{Column, Entity, Model};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, Order, QueryFilter, QueryOrder};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_all<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by(Column::CreatedAt, Order::Desc)
            .all(conn)
            .await
    }

    pub async fn get_latest<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<Option<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by(Column::CreatedAt, Order::Desc)
            .one(conn)
            .await
    }

    pub async fn get_earliest<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<Option<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by(Column::CreatedAt, Order::Asc)
            .one(conn)
            .await
    }

    pub async fn get_all_by_type<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        log_type: &str,
    ) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Type.eq(log_type))
            .order_by(Column::CreatedAt, Order::Desc)
            .all(conn)
            .await
    }

    pub async fn get_latest_by_type<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        log_type: &str,
    ) -> Result<Option<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Type.eq(log_type))
            .order_by(Column::CreatedAt, Order::Desc)
            .one(conn)
            .await
    }

    pub async fn get_earliest_by_type<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        log_type: &str,
    ) -> Result<Option<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Type.eq(log_type))
            .order_by(Column::CreatedAt, Order::Asc)
            .one(conn)
            .await
    }
}
