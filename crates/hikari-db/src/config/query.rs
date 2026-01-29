use hikari_entity::{
    config,
    config::{Entity as UserConfig, Model as UserConfigModel},
};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect, TransactionTrait,
};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_config_value<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        key: &str,
    ) -> Result<Option<String>, DbErr> {
        let res: Result<Option<String>, _> = UserConfig::find()
            .select_only()
            .column(config::Column::Value)
            .filter(
                Condition::all()
                    .add(config::Column::Key.eq(key))
                    .add(config::Column::UserId.eq(user_id)),
            )
            .into_tuple()
            .one(conn)
            .await;
        res
    }

    pub async fn get_user_config<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: Uuid,
    ) -> Result<Vec<UserConfigModel>, DbErr> {
        UserConfig::find()
            .filter(config::Column::UserId.eq(user_id))
            .all(db)
            .await
    }
}
