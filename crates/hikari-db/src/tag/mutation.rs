use hikari_entity::tag::Kind;
use hikari_entity::{tag, tag::Model as TagModel};

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, NotSet, QueryFilter,
};
use std::error::Error;
use uuid::Uuid;

pub struct Mutation;

impl Mutation {
    pub async fn create_focus(
        db: &DatabaseConnection,
        user_id: Option<Uuid>,
        name: String,
        icon: String,
        hidden: bool,
    ) -> Result<TagModel, DbErr> {
        let journal_focus = tag::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            kind: ActiveValue::Set(Kind::Focus),
            user_id: ActiveValue::Set(user_id),
            name: ActiveValue::Set(name),
            icon: ActiveValue::Set(icon),
            hidden: ActiveValue::Set(hidden),
        };
        let focus = journal_focus.insert(db).await;

        if let Err(error) = &focus {
            tracing::error!(error = error as &dyn Error, "failed to create focus");
        }

        focus
    }

    pub async fn update_tag(
        db: &DatabaseConnection,
        focus_id: Uuid,
        user_id: Uuid,
        name: Option<String>,
        icon: Option<String>,
        hidden: Option<bool>,
    ) -> Result<(), DbErr> {
        let tag = tag::ActiveModel {
            id: ActiveValue::Unchanged(Uuid::new_v4()),
            user_id: NotSet,
            kind: NotSet,
            name: if let Some(name) = name {
                ActiveValue::Set(name)
            } else {
                NotSet
            },
            icon: if let Some(icon) = icon {
                ActiveValue::Set(icon)
            } else {
                NotSet
            },
            hidden: if let Some(hidden) = hidden {
                ActiveValue::Set(hidden)
            } else {
                NotSet
            },
        };

        let res = tag::Entity::update_many()
            .set(tag)
            .filter(tag::Column::UserId.eq(user_id))
            .filter(tag::Column::Id.eq(focus_id))
            .exec(db)
            .await?;
        if res.rows_affected == 0 {
            return Err(DbErr::RecordNotFound("Focus entry not found".to_string()));
        }

        Ok(())
    }

    pub async fn create_or_update_global_focus(
        db: &DatabaseConnection,
        name: String,
        icon: String,
        hidden: bool,
    ) -> Result<(), DbErr> {
        let focus = tag::ActiveModel {
            id: NotSet,
            kind: ActiveValue::Set(Kind::Focus),
            user_id: ActiveValue::Set(None),
            name: ActiveValue::Set(name.clone()),
            icon: ActiveValue::Set(icon.clone()),
            hidden: ActiveValue::Set(hidden),
        };
        let res = tag::Entity::update_many()
            .set(focus)
            .filter(tag::Column::Name.eq(&name))
            .filter(tag::Column::UserId.is_null())
            .exec(db)
            .await?;

        if res.rows_affected > 0 {
            return Ok(());
        }

        Self::create_focus(db, None, name, icon, hidden).await?;

        Ok(())
    }
}
