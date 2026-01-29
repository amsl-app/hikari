use std::error::Error;

use hikari_entity::assessment::session::{Entity as AssessmentSessionEntity, Model as AssessmentSession};
use hikari_entity::module::assessment;
use hikari_entity::module::assessment::{
    Column as ModuleAssessmentColumn, Entity as ModuleAssessmentEntity, Model as ModuleAssessment,
};
use sea_orm::prelude::*;
use sea_orm::{JoinType, QuerySelect};

pub struct Query;

impl Query {
    pub async fn all<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<Vec<ModuleAssessment>, DbErr> {
        let res = ModuleAssessmentEntity::find()
            .filter(assessment::Column::UserId.eq(user_id))
            .all(conn)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, "failed to load module assessments");
        })
    }

    pub async fn get_for_module<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        module_id: &str,
    ) -> Result<Option<ModuleAssessment>, DbErr> {
        let res = ModuleAssessmentEntity::find()
            .filter(assessment::Column::UserId.eq(user_id))
            .filter(assessment::Column::Module.eq(module_id))
            .one(db)
            .await;
        res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, %module_id, "failed to get module session instances");
        })
    }

    // pub(crate) fn load_pre_assessment(
    //     conn: &mut SqliteConnection,
    //     user_id: Uuid,
    //     module_key: &str,
    // ) -> Result<(ModuleAssessment, UserAssessmentSession), DbError> {
    //     let query = module_assessment::table
    //         .filter(
    //             module_assessment::module_id
    //                 .eq(module_key)
    //                 .and(module_assessment::user_id.eq(user_id)),
    //         )
    //         .inner_join(
    //             assessment::table.on(module_assessment::last_pre.eq(assessment::id.nullable())),
    //         );
    //
    //     query.get_result(conn).map_err(Into::into)
    // }

    async fn last_linked_assessment<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module_key: &str,
        col: ModuleAssessmentColumn,
    ) -> Result<Option<(AssessmentSession, ModuleAssessment)>, DbErr> {
        let res = ModuleAssessmentEntity::find()
            .filter(assessment::Column::UserId.eq(user_id))
            .filter(assessment::Column::Module.eq(module_key))
            .join(
                JoinType::InnerJoin,
                ModuleAssessmentEntity::belongs_to(hikari_entity::assessment::session::Entity)
                    .from(col)
                    .to(hikari_entity::assessment::session::Column::Id)
                    .into(),
            )
            .select_also(AssessmentSessionEntity)
            .one(conn)
            .await;
        let res = res.inspect_err(|error| {
            tracing::error!(error = error as &dyn Error, %user_id, %module_key, "failed to get assessment instances");
        })?;
        // Assessment and session should always be both some or both none. But we can't express that with sea-orm
        Ok(res.and_then(|(assessment, session)| session.map(|session| (session, assessment))))
    }

    pub async fn load_pre_assessment<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module_key: &str,
    ) -> Result<Option<(AssessmentSession, ModuleAssessment)>, DbErr> {
        Self::last_linked_assessment(conn, user_id, module_key, ModuleAssessmentColumn::LastPre).await
    }

    pub async fn load_post_assessment<C: ConnectionTrait>(
        conn: &C,
        user_id: Uuid,
        module_key: &str,
    ) -> Result<Option<(AssessmentSession, ModuleAssessment)>, DbErr> {
        Self::last_linked_assessment(conn, user_id, module_key, ModuleAssessmentColumn::LastPost).await
    }
}
