use futures_util::future::try_join3;
use hikari_entity::history;
use hikari_entity::history::{history_assessment, history_module, history_session};
use sea_orm::{ConnectionTrait, DbErr};
use uuid::Uuid;

pub struct Query;

pub struct HistoryData {
    pub module: Vec<(history::Model, history_module::Model)>,
    pub session: Vec<(history::Model, history_session::Model)>,
    pub assessment: Vec<(history::Model, history_assessment::Model)>,
}

impl Query {
    pub async fn load_history_entries<C: ConnectionTrait>(conn: &C, user_id: Uuid) -> Result<HistoryData, DbErr> {
        let module_history = super::history_module::Query::get_for_user(conn, user_id);
        let session_history = super::history_session::Query::get_for_user(conn, user_id);
        let assessment_history = super::history_assessment::Query::get_for_user(conn, user_id);

        let (module_history, session_history, assessment_history) =
            try_join3(module_history, session_history, assessment_history).await?;

        Ok(HistoryData {
            module: module_history,
            session: session_history,
            assessment: assessment_history,
        })
    }
}
