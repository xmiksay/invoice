//! Phase 2a: the append-only log of e-mails sent for a document.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE document_emails (
    id          uuid PRIMARY KEY,
    document_id uuid NOT NULL REFERENCES documents (id) ON DELETE CASCADE,
    created_at  timestamptz NOT NULL DEFAULT now(),
    to_addrs    text[] NOT NULL,
    cc_addrs    text[] NOT NULL,
    bcc_addrs   text[] NOT NULL,
    subject     text NOT NULL,
    body        text NOT NULL,
    attachments text[] NOT NULL,
    ok          boolean NOT NULL,
    error       text,
    message_id  text
);
CREATE INDEX document_emails_document_idx ON document_emails (document_id, created_at DESC);
"#;

const DOWN: &str = "DROP TABLE document_emails;";

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
