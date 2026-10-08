//! Phase 1e: received documents (own number series per type, VAT recap
//! entered, metadata), original PDF uploads, manual import, categories and
//! custom field definitions.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
INSERT INTO number_series (doc_type, pattern) VALUES
    ('received_credit_note', 'PD{YYYY}{NNNN}'),
    ('received_proforma', 'PZ{YYYY}{NNNN}'),
    ('received_advance_tax_doc', 'PDP{YYYY}{NNNN}')
ON CONFLICT (doc_type) DO NOTHING;

CREATE TABLE categories (
    id          uuid        PRIMARY KEY,
    name        text        NOT NULL,
    kind        text        NOT NULL CHECK (kind IN ('expense', 'income')),
    active      boolean     NOT NULL DEFAULT true,
    position    integer     NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX categories_kind_name_key ON categories (kind, lower(name));

CREATE TABLE custom_fields (
    id          uuid        PRIMARY KEY,
    key         text        NOT NULL CONSTRAINT custom_fields_key_key UNIQUE,
    label       text        NOT NULL,
    field_type  text        NOT NULL CHECK (field_type IN ('text', 'number', 'date', 'bool', 'select')),
    options     text[]      NOT NULL DEFAULT '{}',
    applies_to  text        NOT NULL CHECK (applies_to IN ('issued', 'received', 'both')),
    required    boolean     NOT NULL DEFAULT false,
    active      boolean     NOT NULL DEFAULT true,
    position    integer     NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

-- A received advance tax document has no due date.
ALTER TABLE documents ALTER COLUMN due_date DROP NOT NULL;
ALTER TABLE documents
    ADD COLUMN supplier_number      text,
    ADD COLUMN received_date        date,
    ADD COLUMN vat_deductible       boolean NOT NULL DEFAULT false,
    ADD COLUMN supplier_account     text,
    -- No ON DELETE action: a category in use cannot be deleted (409).
    ADD COLUMN category_id          uuid REFERENCES categories (id),
    ADD COLUMN custom_fields        jsonb   NOT NULL DEFAULT '{}',
    ADD COLUMN original_path        text,
    ADD COLUMN original_sha256      text,
    ADD COLUMN original_size        bigint,
    ADD COLUMN original_uploaded_at timestamptz;
CREATE INDEX documents_category_idx ON documents (category_id) WHERE category_id IS NOT NULL;
CREATE UNIQUE INDEX documents_received_number_key
    ON documents (doc_type, number) WHERE number IS NOT NULL AND direction = 'received';
"#;

const DOWN: &str = r#"
DELETE FROM documents WHERE direction = 'received';
DROP INDEX documents_received_number_key;
DROP INDEX documents_category_idx;
ALTER TABLE documents
    DROP COLUMN original_uploaded_at,
    DROP COLUMN original_size,
    DROP COLUMN original_sha256,
    DROP COLUMN original_path,
    DROP COLUMN custom_fields,
    DROP COLUMN category_id,
    DROP COLUMN supplier_account,
    DROP COLUMN vat_deductible,
    DROP COLUMN received_date,
    DROP COLUMN supplier_number;
UPDATE documents SET due_date = issue_date WHERE due_date IS NULL;
ALTER TABLE documents ALTER COLUMN due_date SET NOT NULL;
DROP TABLE custom_fields;
DROP TABLE categories;
DELETE FROM number_series
    WHERE doc_type IN ('received_credit_note', 'received_proforma', 'received_advance_tax_doc');
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
