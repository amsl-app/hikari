use futures_util::try_join;
use hikari_entity::access_tokens::{Column as AccessTokenColumn, Entity as AccessToken};
use hikari_entity::custom_groups::{
    Entity as CustomGroupEntity, Model as CustomGroup, Relation as CustomGroupRelations,
};
use hikari_entity::oidc_groups::{Entity as OidcGroupEntity, Model as OidcGroup, Relation as OidcGroupRelations};

use hikari_entity::user::{Entity as UserEntity, Model as User, Relation as UserRelations};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, JoinType, QueryFilter, QuerySelect, RelationTrait};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn find_user_by_id<C: ConnectionTrait>(conn: &C, id: Uuid) -> Result<Option<User>, DbErr> {
        UserEntity::find_by_id(id).one(conn).await.inspect_err(|error| {
            tracing::error!(error = error as &dyn std::error::Error, "error loading user");
        })
    }

    pub async fn find_by_token<C: ConnectionTrait>(
        conn: &C,
        token: &str,
    ) -> Result<Option<(User, Vec<OidcGroup>, Vec<CustomGroup>)>, DbErr> {
        let (user, oidc_groups, custom_groups) = try_join!(
            UserEntity::find()
                .inner_join(AccessToken)
                .filter(AccessTokenColumn::AccessToken.eq(token))
                .one(conn),
            OidcGroupEntity::find()
                .join(JoinType::LeftJoin, OidcGroupRelations::User.def())
                .join(JoinType::LeftJoin, UserRelations::AccessToken.def())
                .filter(AccessTokenColumn::AccessToken.eq(token))
                .all(conn),
            CustomGroupEntity::find()
                .join(JoinType::LeftJoin, CustomGroupRelations::User.def())
                .join(JoinType::LeftJoin, UserRelations::AccessToken.def())
                .filter(AccessTokenColumn::AccessToken.eq(token))
                .all(conn)
        )
        .inspect_err(|error| {
            tracing::error!(error = error as &dyn std::error::Error, "error finding user by token");
        })?;
        Ok(user.map(|user| (user, oidc_groups, custom_groups)))
    }
}
