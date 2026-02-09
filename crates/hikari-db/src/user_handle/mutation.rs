use rand::RngExt;
use hikari_entity::{
    user_handle,
    user_handle::{Entity, Model},
};

use crate::util::FlattenTransactionResultExt;
use sea_orm::prelude::*;

use super::Query;
use sea_orm::{Statement, TransactionError, TransactionTrait};

pub trait HandleGenerator {
    fn generate_handle(len: usize) -> Vec<u8>;
}

pub struct RandomHandleGenerator;

impl HandleGenerator for RandomHandleGenerator {
    fn generate_handle(len: usize) -> Vec<u8> {
        // Security: ThreadRng is a CSPRNG and handles are not sensitive meaning:
        // - The handles should be unpredictable
        // - Even if the handles are predictable, it should not be possible to exploit this
        let mut rng = rand::rng();
        let bytes: Vec<u8> = (&mut rng).sample_iter(rand::distr::StandardUniform).take(len).collect();

        bytes
    }
}

pub struct Mutation;

impl Mutation {
    /// Gets the user handle for a user, or creates one if it doesn't exist
    pub async fn get_or_create_handle<Rng: HandleGenerator, C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        user_id: Uuid,
        mut len: usize,
    ) -> Result<Model, DbErr> {
        let user_handles = Query::get_for_user(conn, user_id).await?;
        if let Some(user_handle) = user_handles.first() {
            return Ok(user_handle.clone());
        }

        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration % 3 == 0 {
                len += 1;
            }

            let res = conn
                .transaction(|txn| {
                    Box::pin(async move {
                        let handle = Rng::generate_handle(len);
                        // Can't use returning because of how sea_orm works
                        txn.execute(Statement::from_sql_and_values(
                            txn.get_database_backend(),
                            r#"
INSERT INTO "user_handle" ("handle", "user_id")
select * from (values ($1, $2)) as new_values
where not exists (
    select * from "user_handle" where "user_id" = $2
)"#,
                            vec![handle.into(), user_id.into()],
                        ))
                        .await?;
                        let token = Entity::find()
                            .filter(user_handle::Column::UserId.eq(user_id))
                            .one(txn)
                            .await?;
                        token.ok_or(DbErr::RecordNotFound("Token not found after insertion".to_owned()))
                    })
                })
                .await;
            let res = match res {
                Ok(token) => Ok(token),
                Err(TransactionError::Transaction(DbErr::Exec(RuntimeErr::SqlxError(
                    sqlx::error::Error::Database(error),
                )))) if error.kind() == sqlx::error::ErrorKind::UniqueViolation => {
                    // Handle already exists - try again
                    continue;
                }
                Err(e) => Err(e),
            };
            return res.flatten_res();
        }
    }
}
