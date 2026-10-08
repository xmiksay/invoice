//! Database migrations (sea-orm-migration).
//!
//! **Adding a migration:** create `src/migration/mYYYYMMDD_NNNNNN_<name>.rs`
//! implementing `MigrationTrait`, declare it with `mod` below and append
//! `Box::new(<module>::Migration)` to [`Migrator::migrations`]. Migrations are
//! append-only — never edit one that has shipped.

pub use sea_orm_migration::prelude::*;

mod m20261008_000001_settings;
mod m20261008_000002_contacts;
mod m20261009_000001_documents;
mod m20261010_000001_phase_1c;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261008_000001_settings::Migration),
            Box::new(m20261008_000002_contacts::Migration),
            Box::new(m20261009_000001_documents::Migration),
            Box::new(m20261010_000001_phase_1c::Migration),
        ]
    }
}
