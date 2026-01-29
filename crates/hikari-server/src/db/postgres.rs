use diesel_migrations::EmbeddedMigrations;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/postgres");

#[cfg(test)]
mod tests;
