//! Contacts: one address book for customers and suppliers.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE contacts (
    id               uuid        PRIMARY KEY,
    name             text        NOT NULL,
    ico              text,
    dic              text,
    street           text        NOT NULL DEFAULT '',
    city             text        NOT NULL DEFAULT '',
    zip              text        NOT NULL DEFAULT '',
    country          text        NOT NULL DEFAULT 'CZ',
    email            text,
    phone            text,
    note             text,
    default_due_days integer,
    default_locale   text,
    default_currency text,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX contacts_ico_key ON contacts (ico) WHERE ico IS NOT NULL;
CREATE INDEX contacts_name_idx ON contacts (name);
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE contacts;")
            .await?;
        Ok(())
    }
}
