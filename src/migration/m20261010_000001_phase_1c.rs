//! Phase 1c: DDPP ↔ payment link, credit-note reason, copied ("original")
//! exchange rates, advance lines, catalog.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
ALTER TABLE documents
    ADD COLUMN payment_id uuid UNIQUE REFERENCES payments (id) ON DELETE SET NULL,
    ADD COLUMN correction_reason text;
ALTER TABLE documents DROP CONSTRAINT documents_exchange_rate_source_check;
ALTER TABLE documents ADD CONSTRAINT documents_exchange_rate_source_check
    CHECK (exchange_rate_source IN ('cnb', 'manual', 'original'));
CREATE INDEX documents_related_idx ON documents (related_document_id)
    WHERE related_document_id IS NOT NULL;

ALTER TABLE document_lines DROP CONSTRAINT document_lines_kind_check;
ALTER TABLE document_lines ADD CONSTRAINT document_lines_kind_check
    CHECK (kind IN ('item', 'text', 'subtotal', 'advance'));
-- The deducted document is issued (never deleted), so no ON DELETE action.
ALTER TABLE document_lines
    ADD COLUMN advance_document_id uuid REFERENCES documents (id),
    -- Deducted amounts per rate, frozen on save / issue (JSON array).
    ADD COLUMN advance_recap jsonb;
CREATE INDEX document_lines_advance_idx ON document_lines (advance_document_id)
    WHERE advance_document_id IS NOT NULL;

CREATE TABLE catalog_items (
    id          uuid          PRIMARY KEY,
    name        text          NOT NULL,
    unit        text,
    unit_price  numeric(18,4) NOT NULL,
    currency    text          NOT NULL,
    vat_rate    numeric(5,2)  NOT NULL,
    active      boolean       NOT NULL DEFAULT true,
    note        text,
    created_at  timestamptz   NOT NULL DEFAULT now(),
    updated_at  timestamptz   NOT NULL DEFAULT now()
);

CREATE TABLE catalog_groups (
    id          uuid        PRIMARY KEY,
    name        text        NOT NULL,
    collapse    boolean     NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE catalog_group_members (
    group_id  uuid          NOT NULL REFERENCES catalog_groups (id) ON DELETE CASCADE,
    item_id   uuid          NOT NULL REFERENCES catalog_items (id) ON DELETE CASCADE,
    position  integer       NOT NULL,
    quantity  numeric(18,4) NOT NULL,
    PRIMARY KEY (group_id, item_id),
    UNIQUE (group_id, position)
);
CREATE INDEX catalog_group_members_item_idx ON catalog_group_members (item_id);
"#;

const DOWN: &str = r#"
DROP TABLE catalog_group_members;
DROP TABLE catalog_groups;
DROP TABLE catalog_items;
DELETE FROM document_lines WHERE kind = 'advance';
ALTER TABLE document_lines DROP COLUMN advance_recap, DROP COLUMN advance_document_id;
ALTER TABLE document_lines DROP CONSTRAINT document_lines_kind_check;
ALTER TABLE document_lines ADD CONSTRAINT document_lines_kind_check
    CHECK (kind IN ('item', 'text', 'subtotal'));
DROP INDEX documents_related_idx;
UPDATE documents SET exchange_rate_source = 'manual' WHERE exchange_rate_source = 'original';
ALTER TABLE documents DROP CONSTRAINT documents_exchange_rate_source_check;
ALTER TABLE documents ADD CONSTRAINT documents_exchange_rate_source_check
    CHECK (exchange_rate_source IN ('cnb', 'manual'));
ALTER TABLE documents DROP COLUMN correction_reason, DROP COLUMN payment_id;
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
