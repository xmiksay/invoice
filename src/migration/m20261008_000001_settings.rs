//! Settings: company profile (singleton), bank accounts, VAT rates, number series.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE company (
    id               smallint    PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    name             text        NOT NULL DEFAULT '',
    ico              text,
    dic              text,
    vat_payer        boolean     NOT NULL DEFAULT false,
    street           text        NOT NULL DEFAULT '',
    city             text        NOT NULL DEFAULT '',
    zip              text        NOT NULL DEFAULT '',
    country          text        NOT NULL DEFAULT 'CZ',
    email            text,
    phone            text,
    web              text,
    registration     text,
    default_due_days integer     NOT NULL DEFAULT 14,
    default_locale   text        NOT NULL DEFAULT 'cs',
    updated_at       timestamptz NOT NULL DEFAULT now()
);
INSERT INTO company (id) VALUES (1);

CREATE TABLE bank_accounts (
    id             uuid        PRIMARY KEY,
    label          text,
    currency       text        NOT NULL,
    account_number text,
    iban           text,
    bic            text,
    is_default     boolean     NOT NULL DEFAULT false,
    created_at     timestamptz NOT NULL DEFAULT now(),
    CHECK (account_number IS NOT NULL OR iban IS NOT NULL)
);
CREATE UNIQUE INDEX bank_accounts_one_default_per_currency
    ON bank_accounts (currency) WHERE is_default;

CREATE TABLE vat_rates (
    id         uuid         PRIMARY KEY,
    rate       numeric(5,2) NOT NULL CHECK (rate >= 0 AND rate <= 100),
    label      text         NOT NULL,
    is_default boolean      NOT NULL DEFAULT false,
    active     boolean      NOT NULL DEFAULT true,
    position   integer      NOT NULL DEFAULT 0,
    CONSTRAINT vat_rates_rate_key UNIQUE (rate)
);
CREATE UNIQUE INDEX vat_rates_one_default ON vat_rates ((true)) WHERE is_default;
INSERT INTO vat_rates (id, rate, label, is_default, position) VALUES
    (gen_random_uuid(), 21, 'Základní', true, 1),
    (gen_random_uuid(), 12, 'Snížená', false, 2),
    (gen_random_uuid(), 0, 'Nulová', false, 3);

CREATE TABLE number_series (
    doc_type text PRIMARY KEY,
    pattern  text NOT NULL
);
INSERT INTO number_series (doc_type, pattern) VALUES
    ('invoice', '{YYYY}{NNNN}'),
    ('credit_note', 'D{YYYY}{NNNN}'),
    ('proforma', 'Z{YYYY}{NNNN}'),
    ('advance_tax_doc', 'DP{YYYY}{NNNN}'),
    ('received', 'P{YYYY}{NNNN}');

CREATE TABLE number_series_counters (
    doc_type    text    NOT NULL REFERENCES number_series (doc_type) ON DELETE CASCADE,
    year        integer NOT NULL,
    last_number integer NOT NULL CHECK (last_number >= 0),
    PRIMARY KEY (doc_type, year)
);
"#;

const DOWN: &str = r#"
DROP TABLE number_series_counters;
DROP TABLE number_series;
DROP TABLE vat_rates;
DROP TABLE bank_accounts;
DROP TABLE company;
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
