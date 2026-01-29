use hikari_model::status::ComponentStatus;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::Query;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::error::Error;
use std::time::Duration;
use tokio::time::timeout;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn get_sea_orm_db_status(conn: &DatabaseConnection, duration: Option<Duration>) -> ComponentStatus {
    let mut query = Query::select();
    query.expr(Expr::current_timestamp());
    timeout(
        duration.unwrap_or_else(|| Duration::from_secs(5)),
        conn.execute(conn.get_database_backend().build(&query)),
    )
    .await
    .inspect_err(|error| tracing::error!(error = error as &dyn Error, "db error during health check"))
    .into()
}
