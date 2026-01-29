use uuid::Uuid;

pub struct ModuleAssessmentInstance {
    pub user_id: Uuid,
    pub module: String,
    pub last_pre: Option<Uuid>,
    pub last_post: Option<Uuid>,
}
