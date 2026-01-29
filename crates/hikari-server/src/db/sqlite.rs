use diesel_migrations::EmbeddedMigrations;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");

#[cfg(test)]
mod tests;
