use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "planner_goal_milestone")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub goal_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub milestone_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::planner_goal::Entity",
        from = "Column::GoalId",
        to = "super::planner_goal::Column::Id"
    )]
    Goal,
    #[sea_orm(
        belongs_to = "super::planner_milestone::Entity",
        from = "Column::MilestoneId",
        to = "super::planner_milestone::Column::Id"
    )]
    Milestone,
}

impl Related<super::planner_goal::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Goal.def()
    }
}

impl Related<super::planner_milestone::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Milestone.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
