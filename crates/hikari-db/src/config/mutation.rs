use hikari_entity::{config, config::Entity as UserConfig};
use sea_orm::{
    ActiveValue, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, DbErr, DeleteResult, EntityTrait,
    InsertResult, QueryFilter, sea_query,
};
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn set_config_value<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        key: String,
        value: String,
    ) -> Result<InsertResult<config::ActiveModel>, DbErr> {
        let config_value = config::ActiveModel {
            user_id: ActiveValue::Set(user_id),
            key: ActiveValue::Set(key),
            value: ActiveValue::Set(value),
        };

        UserConfig::insert(config_value)
            .on_conflict(
                sea_query::OnConflict::columns([config::Column::UserId, config::Column::Key])
                    .update_column(config::Column::Value)
                    .clone(),
            )
            .exec(conn)
            .await
    }

    pub async fn delete_config_value(
        db: &DatabaseConnection,
        user_id: Uuid,
        key: String,
    ) -> Result<DeleteResult, DbErr> {
        UserConfig::delete_many()
            .filter(
                Condition::all()
                    .add(config::Column::Key.eq(key))
                    .add(config::Column::UserId.eq(user_id)),
            )
            .exec(db)
            .await
    }
}
