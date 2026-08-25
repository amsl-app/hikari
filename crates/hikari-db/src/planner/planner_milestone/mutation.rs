use chrono::NaiveDate;
use hikari_entity::planner_milestone::{
    ActiveModel, Column, Entity as PlannerMilestone, Model as PlannerMilestoneModel,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, NotSet, QueryFilter, sea_query,
};
use uuid::Uuid;

pub struct MilestoneInput {
    pub title: String,
    pub date: NaiveDate,
    pub description: Option<String>,
    pub module_id: Option<String>,
    pub origin_id: Option<String>,
}

pub struct Mutation;

impl Mutation {
    pub async fn create_milestone<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        input: MilestoneInput,
    ) -> Result<PlannerMilestoneModel, DbErr> {
        let model = ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            user_id: ActiveValue::Set(user_id),
            title: ActiveValue::Set(input.title),
            date: ActiveValue::Set(input.date),
            description: ActiveValue::Set(input.description),
            module_id: ActiveValue::Set(input.module_id),
            origin_id: ActiveValue::Set(input.origin_id),
            created_at: NotSet,
            updated_at: NotSet,
        };
        model
            .insert(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to create milestone"))
    }

    pub async fn update_milestone<C: ConnectionTrait>(
        db: &C,
        mut active_model: ActiveModel,
    ) -> Result<PlannerMilestoneModel, DbErr> {
        active_model.updated_at = ActiveValue::Set(chrono::Utc::now().naive_utc());
        active_model
            .update(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to update milestone"))
    }

    pub async fn delete_milestone<C: ConnectionTrait>(db: &C, user_id: Uuid, id: Uuid) -> Result<u64, DbErr> {
        let res = PlannerMilestone::delete_many()
            .filter(Column::Id.eq(id))
            .filter(Column::UserId.eq(user_id))
            .exec(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to delete milestone"))?;
        Ok(res.rows_affected)
    }

    pub async fn import_milestones<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        inputs: Vec<MilestoneInput>,
    ) -> Result<Vec<PlannerMilestoneModel>, DbErr> {
        if inputs.is_empty() {
            return Ok(vec![]);
        }
        let ids: Vec<Uuid> = (0..inputs.len()).map(|_| Uuid::new_v4()).collect();
        let models = ids.iter().zip(inputs).map(|(id, input)| ActiveModel {
            id: ActiveValue::Set(*id),
            user_id: ActiveValue::Set(user_id),
            title: ActiveValue::Set(input.title),
            date: ActiveValue::Set(input.date),
            description: ActiveValue::Set(input.description),
            module_id: ActiveValue::Set(input.module_id),
            origin_id: ActiveValue::Set(input.origin_id),
            created_at: NotSet,
            updated_at: NotSet,
        });
        PlannerMilestone::insert_many(models)
            .on_conflict(
                sea_query::OnConflict::columns([Column::UserId, Column::ModuleId, Column::OriginId])
                    .do_nothing()
                    .to_owned(),
            )
            .exec(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to import milestones"))?;
        PlannerMilestone::find()
            .filter(Column::Id.is_in(ids))
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await
            .inspect_err(|error| tracing::error!(%error, "failed to fetch imported milestones"))
    }
}
