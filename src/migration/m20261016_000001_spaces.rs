//! Phase 4a: spaces, users, memberships, sessions, personal API tokens and
//! e-mail verification / password reset tokens; `space_id` on every root
//! table, unique indexes per space. The former global seeds (company row,
//! VAT rates, number series, accounting settings) are removed here and
//! created per space by `space::seed` instead.
//!
//! There is no default space to move existing data into: the migration
//! refuses to run while any space-owned table holds user data.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const GUARD: &str = r#"
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM documents) OR EXISTS (SELECT 1 FROM contacts)
        OR EXISTS (SELECT 1 FROM bank_accounts) OR EXISTS (SELECT 1 FROM catalog_items)
        OR EXISTS (SELECT 1 FROM catalog_groups) OR EXISTS (SELECT 1 FROM categories)
        OR EXISTS (SELECT 1 FROM custom_fields) OR EXISTS (SELECT 1 FROM number_series_counters)
        OR EXISTS (SELECT 1 FROM company WHERE name <> '' OR ico IS NOT NULL)
    THEN
        RAISE EXCEPTION 'phase 4a (spaces) cannot migrate existing data: there is no default space. The database holds documents, contacts or settings - reset it (drop and recreate the database) before upgrading.';
    END IF;
END
$$;
"#;

const UP: &str = r#"
CREATE TABLE spaces (
    id         uuid        PRIMARY KEY,
    slug       text        NOT NULL CONSTRAINT spaces_slug_key UNIQUE,
    name       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE users (
    id                uuid        PRIMARY KEY,
    email             text        NOT NULL CONSTRAINT users_email_key UNIQUE,
    display_name      text        NOT NULL,
    password_hash     text        NOT NULL,
    email_verified_at timestamptz,
    disabled          boolean     NOT NULL DEFAULT false,
    created_at        timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE space_members (
    space_id   uuid        NOT NULL REFERENCES spaces (id) ON DELETE CASCADE,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role       text        NOT NULL CHECK (role IN ('owner', 'admin', 'member', 'accountant')),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (space_id, user_id)
);
CREATE INDEX space_members_user_idx ON space_members (user_id);

CREATE TABLE sessions (
    id           uuid        PRIMARY KEY,
    token_hash   text        NOT NULL CONSTRAINT sessions_token_hash_key UNIQUE,
    user_id      uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- NULL: a session of the base host.
    space_id     uuid        REFERENCES spaces (id) ON DELETE CASCADE,
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL,
    user_agent   text
);
CREATE INDEX sessions_user_idx ON sessions (user_id);

CREATE TABLE api_tokens (
    id           uuid        PRIMARY KEY,
    space_id     uuid        NOT NULL REFERENCES spaces (id) ON DELETE CASCADE,
    user_id      uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name         text        NOT NULL,
    prefix       text        NOT NULL,
    token_hash   text        NOT NULL CONSTRAINT api_tokens_token_hash_key UNIQUE,
    role         text        NOT NULL CHECK (role IN ('owner', 'admin', 'member', 'accountant')),
    created_at   timestamptz NOT NULL DEFAULT now(),
    -- Last valid day (UTC); NULL = no expiry.
    expires_at   date,
    last_used_at timestamptz
);
CREATE INDEX api_tokens_space_idx ON api_tokens (space_id, created_at);

CREATE TABLE user_tokens (
    id         uuid        PRIMARY KEY,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind       text        NOT NULL CHECK (kind IN ('verify', 'reset')),
    token_hash text        NOT NULL CONSTRAINT user_tokens_token_hash_key UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    used_at    timestamptz
);
CREATE INDEX user_tokens_user_idx ON user_tokens (user_id, kind);

-- Singletons become one row per space.
DELETE FROM company;
ALTER TABLE company DROP COLUMN id;
ALTER TABLE company ADD COLUMN space_id uuid PRIMARY KEY REFERENCES spaces (id) ON DELETE CASCADE;

DELETE FROM accounting_settings;
ALTER TABLE accounting_settings DROP COLUMN id;
ALTER TABLE accounting_settings
    ADD COLUMN space_id uuid PRIMARY KEY REFERENCES spaces (id) ON DELETE CASCADE;

-- Number series get a surrogate key so counters stay children of their series.
DROP TABLE number_series_counters;
DROP TABLE number_series;
CREATE TABLE number_series (
    id       uuid PRIMARY KEY,
    space_id uuid NOT NULL REFERENCES spaces (id) ON DELETE CASCADE,
    doc_type text NOT NULL,
    pattern  text NOT NULL,
    CONSTRAINT number_series_space_doc_type_key UNIQUE (space_id, doc_type)
);
CREATE TABLE number_series_counters (
    series_id   uuid    NOT NULL REFERENCES number_series (id) ON DELETE CASCADE,
    year        integer NOT NULL,
    last_number integer NOT NULL CHECK (last_number >= 0),
    PRIMARY KEY (series_id, year)
);

DELETE FROM vat_rates;
ALTER TABLE vat_rates DROP CONSTRAINT vat_rates_rate_key;
DROP INDEX vat_rates_one_default;
ALTER TABLE vat_rates ADD COLUMN space_id uuid NOT NULL REFERENCES spaces (id) ON DELETE CASCADE;
ALTER TABLE vat_rates ADD CONSTRAINT vat_rates_rate_key UNIQUE (space_id, rate);
CREATE UNIQUE INDEX vat_rates_one_default ON vat_rates (space_id) WHERE is_default;

ALTER TABLE bank_accounts ADD COLUMN space_id uuid NOT NULL REFERENCES spaces (id) ON DELETE CASCADE;
DROP INDEX bank_accounts_one_default_per_currency;
CREATE UNIQUE INDEX bank_accounts_one_default_per_currency
    ON bank_accounts (space_id, currency) WHERE is_default;

ALTER TABLE contacts ADD COLUMN space_id uuid NOT NULL REFERENCES spaces (id) ON DELETE CASCADE;
DROP INDEX contacts_ico_key;
CREATE UNIQUE INDEX contacts_ico_key ON contacts (space_id, ico) WHERE ico IS NOT NULL;
DROP INDEX contacts_name_idx;
CREATE INDEX contacts_name_idx ON contacts (space_id, name);

ALTER TABLE categories ADD COLUMN space_id uuid NOT NULL REFERENCES spaces (id) ON DELETE CASCADE;
DROP INDEX categories_kind_name_key;
CREATE UNIQUE INDEX categories_kind_name_key ON categories (space_id, kind, lower(name));

ALTER TABLE custom_fields DROP CONSTRAINT custom_fields_key_key;
ALTER TABLE custom_fields ADD COLUMN space_id uuid NOT NULL REFERENCES spaces (id) ON DELETE CASCADE;
ALTER TABLE custom_fields ADD CONSTRAINT custom_fields_key_key UNIQUE (space_id, key);

ALTER TABLE catalog_items ADD COLUMN space_id uuid NOT NULL REFERENCES spaces (id) ON DELETE CASCADE;
CREATE INDEX catalog_items_space_idx ON catalog_items (space_id, name);
ALTER TABLE catalog_groups ADD COLUMN space_id uuid NOT NULL REFERENCES spaces (id) ON DELETE CASCADE;
CREATE INDEX catalog_groups_space_idx ON catalog_groups (space_id, name);

ALTER TABLE documents ADD COLUMN space_id uuid NOT NULL REFERENCES spaces (id) ON DELETE CASCADE;
DROP INDEX documents_issued_number_key;
CREATE UNIQUE INDEX documents_issued_number_key
    ON documents (space_id, doc_type, number) WHERE number IS NOT NULL AND direction = 'issued';
DROP INDEX documents_received_number_key;
CREATE UNIQUE INDEX documents_received_number_key
    ON documents (space_id, doc_type, number) WHERE number IS NOT NULL AND direction = 'received';
DROP INDEX documents_series_idx;
CREATE INDEX documents_series_idx ON documents (space_id, doc_type, number_year)
    WHERE number_seq IS NOT NULL;
DROP INDEX documents_issue_date_idx;
CREATE INDEX documents_issue_date_idx ON documents (space_id, issue_date DESC, created_at DESC);
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(GUARD).await?;
        db.execute_unprepared(UP).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "phase 4a (spaces) cannot be rolled back: per-space data has no single-tenant \
             form; restore a backup instead"
                .into(),
        ))
    }
}
