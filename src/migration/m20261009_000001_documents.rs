//! Documents (header, lines, stored VAT recap), payments, ČNB exchange-rate cache.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE documents (
    id                   uuid          PRIMARY KEY,
    direction            text          NOT NULL CHECK (direction IN ('issued', 'received')),
    doc_type             text          NOT NULL CHECK (doc_type IN
                             ('invoice', 'credit_note', 'proforma', 'advance_tax_doc', 'received')),
    status               text          NOT NULL DEFAULT 'draft'
                             CHECK (status IN ('draft', 'issued', 'cancelled')),
    number               text,
    number_year          integer,
    number_seq           integer,
    imported             boolean       NOT NULL DEFAULT false,
    contact_id           uuid          REFERENCES contacts (id) ON DELETE SET NULL,
    issue_date           date          NOT NULL,
    tax_point_date       date,
    due_date             date          NOT NULL,
    currency             text          NOT NULL,
    exchange_rate        numeric(18,6),
    exchange_rate_date   date,
    exchange_rate_source text          CHECK (exchange_rate_source IN ('cnb', 'manual')),
    locale               text          NOT NULL,
    vat_mode             text          NOT NULL
                             CHECK (vat_mode IN ('standard', 'reverse_charge', 'exempt', 'non_payer')),
    bank_account_id      uuid          REFERENCES bank_accounts (id) ON DELETE SET NULL,
    payment_method       text          NOT NULL
                             CHECK (payment_method IN ('bank_transfer', 'cash', 'card', 'other')),
    variable_symbol      text,
    constant_symbol      text,
    order_ref            text,
    header_note          text,
    footer_note          text,
    internal_note        text,
    round_total          boolean       NOT NULL DEFAULT false,
    supplier_snapshot    jsonb,
    customer_snapshot    jsonb,
    bank_snapshot        jsonb,
    total_base           numeric(18,2) NOT NULL DEFAULT 0,
    total_vat            numeric(18,2) NOT NULL DEFAULT 0,
    total                numeric(18,2) NOT NULL DEFAULT 0,
    rounding             numeric(18,2) NOT NULL DEFAULT 0,
    payable              numeric(18,2) NOT NULL DEFAULT 0,
    total_czk            numeric(18,2),
    paid                 numeric(18,2) NOT NULL DEFAULT 0,
    sent_at              timestamptz,
    cancelled_at         timestamptz,
    cancel_reason        text,
    related_document_id  uuid          REFERENCES documents (id) ON DELETE SET NULL,
    created_at           timestamptz   NOT NULL DEFAULT now(),
    updated_at           timestamptz   NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX documents_issued_number_key
    ON documents (doc_type, number) WHERE number IS NOT NULL AND direction = 'issued';
CREATE INDEX documents_series_idx ON documents (doc_type, number_year) WHERE number_seq IS NOT NULL;
CREATE INDEX documents_issue_date_idx ON documents (issue_date DESC, created_at DESC);
CREATE INDEX documents_contact_idx ON documents (contact_id);

CREATE TABLE document_lines (
    id           uuid          PRIMARY KEY,
    document_id  uuid          NOT NULL REFERENCES documents (id) ON DELETE CASCADE,
    position     integer       NOT NULL,
    kind         text          NOT NULL CHECK (kind IN ('item', 'text', 'subtotal')),
    description  text          NOT NULL DEFAULT '',
    quantity     numeric(18,4),
    unit         text,
    unit_price   numeric(18,4),
    discount_pct numeric(5,2),
    vat_rate     numeric(5,2),
    refs         integer[],
    collapse     boolean       NOT NULL DEFAULT false,
    UNIQUE (document_id, position)
);

CREATE TABLE document_vat_recap (
    document_id uuid          NOT NULL REFERENCES documents (id) ON DELETE CASCADE,
    vat_rate    numeric(5,2)  NOT NULL,
    base        numeric(18,2) NOT NULL,
    vat         numeric(18,2) NOT NULL,
    base_czk    numeric(18,2),
    vat_czk     numeric(18,2),
    PRIMARY KEY (document_id, vat_rate)
);

CREATE TABLE payments (
    id          uuid          PRIMARY KEY,
    document_id uuid          NOT NULL REFERENCES documents (id) ON DELETE CASCADE,
    date        date          NOT NULL,
    amount      numeric(18,2) NOT NULL CHECK (amount > 0),
    note        text,
    created_at  timestamptz   NOT NULL DEFAULT now()
);
CREATE INDEX payments_document_idx ON payments (document_id, date);

CREATE TABLE exchange_rates (
    currency       text          NOT NULL,
    requested_date date          NOT NULL,
    published_date date          NOT NULL,
    -- NULL: ČNB's list for that date does not contain the currency.
    rate           numeric(18,6) CHECK (rate > 0),
    PRIMARY KEY (currency, requested_date)
);
"#;

const DOWN: &str = r#"
DROP TABLE exchange_rates;
DROP TABLE payments;
DROP TABLE document_vat_recap;
DROP TABLE document_lines;
DROP TABLE documents;
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
