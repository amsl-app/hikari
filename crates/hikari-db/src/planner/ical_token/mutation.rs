use crate::util::generate_url_safe_token;
use hikari_entity::planner_ical_token::{ActiveModel, Column, Entity as IcalToken, Model};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::*;
use sea_orm::sea_query;

pub struct Mutation;

impl Mutation {
    pub async fn get_or_create_ical_token<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Model, DbErr> {
        let token = generate_url_safe_token();
        let active = ActiveModel {
            user_id: Set(user_id),
            token: Set(token),
            ..Default::default()
        };

        IcalToken::insert(active)
            .on_conflict(sea_query::OnConflict::column(Column::UserId).do_nothing().to_owned())
            .exec_without_returning(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = %error, "failed to create ical token");
            })?;

        IcalToken::find_by_id(user_id)
            .one(db)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("ical token not found after get_or_create".to_owned()))
    }

    pub async fn delete_ical_token<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<(), DbErr> {
        IcalToken::delete_many()
            .filter(Column::UserId.eq(user_id))
            .exec(db)
            .await
            .inspect_err(|error| {
                tracing::error!(error = %error, "failed to delete ical token");
            })?;
        Ok(())
    }
}
