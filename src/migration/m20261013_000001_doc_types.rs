//! Phase 1f-a: debit notes, DDPP corrections and simplified tax documents —
//! the `documents.doc_type` CHECK and their number series (both directions).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
ALTER TABLE documents DROP CONSTRAINT documents_doc_type_check;
ALTER TABLE documents ADD CONSTRAINT documents_doc_type_check
    CHECK (doc_type IN ('invoice', 'credit_note', 'debit_note', 'proforma', 'advance_tax_doc',
                        'advance_credit_note', 'simplified', 'received'));

INSERT INTO number_series (doc_type, pattern) VALUES
    ('debit_note', 'V{YYYY}{NNNN}'),
    ('advance_credit_note', 'OP{YYYY}{NNNN}'),
    ('simplified', 'ZD{YYYY}{NNNN}'),
    ('received_debit_note', 'PV{YYYY}{NNNN}'),
    ('received_advance_credit_note', 'POP{YYYY}{NNNN}'),
    ('received_simplified', 'PZD{YYYY}{NNNN}')
ON CONFLICT (doc_type) DO NOTHING;
"#;

const DOWN: &str = r#"
DELETE FROM documents WHERE doc_type IN ('debit_note', 'advance_credit_note', 'simplified');
DELETE FROM number_series WHERE doc_type IN ('debit_note', 'advance_credit_note', 'simplified',
    'received_debit_note', 'received_advance_credit_note', 'received_simplified');
ALTER TABLE documents DROP CONSTRAINT documents_doc_type_check;
ALTER TABLE documents ADD CONSTRAINT documents_doc_type_check
    CHECK (doc_type IN ('invoice', 'credit_note', 'proforma', 'advance_tax_doc', 'received'));
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
