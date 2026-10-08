//! Phase 1d: the archived PDF of an issued document.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
ALTER TABLE documents
    ADD COLUMN pdf_path text,
    ADD COLUMN pdf_sha256 text,
    ADD COLUMN pdf_rendered_at timestamptz;
"#;

const DOWN: &str = r#"
ALTER TABLE documents
    DROP COLUMN pdf_rendered_at,
    DROP COLUMN pdf_sha256,
    DROP COLUMN pdf_path;
"#;

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
