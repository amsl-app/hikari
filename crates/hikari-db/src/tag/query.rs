use hikari_entity::journal::journal_entry;
use hikari_entity::tag;
use hikari_entity::tag::{Entity as Tag, Model as TagModel};

use sea_orm::{ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use std::error::Error;
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_user_journal_entry_focus<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        journal_entry_id: Uuid,
    ) -> Result<Vec<TagModel>, DbErr> {
        let focus: Result<Vec<TagModel>, DbErr> = Tag::find()
            .left_join(journal_entry::Entity)
            .filter(journal_entry::Column::Id.eq(journal_entry_id))
            .filter(
                Condition::any()
                    .add(journal_entry::Column::UserId.eq(user_id))
                    .add(journal_entry::Column::UserId.is_null()),
            )
            .filter(tag::Column::Kind.eq(tag::Kind::Focus))
            .all(conn)
            .await;

        if let Err(error) = &focus {
            tracing::error!(error = error as &dyn Error, "failed to load journal entry focuses");
        }

        focus
    }

    pub async fn get_user_focuses<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        include_global: bool,
    ) -> Result<Vec<TagModel>, DbErr> {
        let focus: Result<Vec<TagModel>, DbErr> = Tag::find()
            .filter({
                let mut filter = Condition::any().add(tag::Column::UserId.eq(user_id));
                if include_global {
                    filter = filter.add(tag::Column::UserId.is_null());
                }
                filter
            })
            .filter(tag::Column::Kind.eq(tag::Kind::Focus))
            .all(conn)
            .await;

        if let Err(error) = &focus {
            tracing::error!(error = error as &dyn Error, "failed to load user focuses");
        }

        focus
    }

    pub async fn get_user_focus<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        focus_id: Uuid,
    ) -> Result<Option<TagModel>, DbErr> {
        Tag::find_by_id(focus_id)
            .filter({
                Condition::any()
                    .add(tag::Column::UserId.eq(user_id))
                    .add(tag::Column::UserId.is_null())
            })
            .filter(tag::Column::Kind.eq(tag::Kind::Focus))
            .one(conn)
            .await
    }
}
