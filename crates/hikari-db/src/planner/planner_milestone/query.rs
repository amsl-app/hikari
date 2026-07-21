use hikari_entity::planner_milestone::{Column, Entity as PlannerMilestone, Model as PlannerMilestoneModel};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

pub struct Query;

impl Query {
    pub async fn get_user_milestones<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
    ) -> Result<Vec<PlannerMilestoneModel>, DbErr> {
        PlannerMilestone::find()
            .filter(Column::UserId.eq(user_id))
            .order_by_asc(Column::Date)
            .all(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to load user milestones"))
    }

    pub async fn get_user_milestone<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Option<PlannerMilestoneModel>, DbErr> {
        PlannerMilestone::find_by_id(id)
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to load user milestone"))
    }

    pub async fn get_imported_origin_ids<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        module_id: &str,
    ) -> Result<Vec<String>, DbErr> {
        let rows = PlannerMilestone::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::ModuleId.eq(module_id))
            .filter(Column::OriginId.is_not_null())
            .all(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to load imported origin ids"))?;
        Ok(rows.into_iter().filter_map(|m| m.origin_id).collect())
    }

    pub async fn get_milestones_by_ids<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        ids: Vec<Uuid>,
    ) -> Result<Vec<PlannerMilestoneModel>, DbErr> {
        let len = ids.len();
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let res = PlannerMilestone::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Id.is_in(ids))
            .all(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to load milestones by ids"))?;

        if res.len() != len {
            Err(DbErr::RecordNotFound("one or more milestone ids do not exist".to_owned(),))
        } else {
            Ok(res)
        }
    }
}
