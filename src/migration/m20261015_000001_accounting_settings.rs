//! Phase 3b: Settings → Accounting, one JSON row (codes for the accounting
//! program exports).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE accounting_settings (
    id         smallint    PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    data       jsonb       NOT NULL DEFAULT '{}',
    updated_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO accounting_settings (id) VALUES (1);
"#;

const DOWN: &str = "DROP TABLE accounting_settings;";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(DOWN).await?;
        Ok(())
    }
}
