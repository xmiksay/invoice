//! Database migrations (sea-orm-migration).
//!
//! **Adding a migration:** create `src/migration/mYYYYMMDD_NNNNNN_<name>.rs`
//! implementing `MigrationTrait`, declare it with `mod` below and append
//! `Box::new(<module>::Migration)` to [`Migrator::migrations`]. Migrations are
//! append-only — never edit one that has shipped.

pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}
