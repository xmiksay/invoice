//! Phase 4b: pending invitations into a space (`docs/api/members.md`). One
//! per (space, e-mail): a new invitation replaces the pending one.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r#"
CREATE TABLE space_invites (
    id         uuid        PRIMARY KEY,
    space_id   uuid        NOT NULL REFERENCES spaces (id) ON DELETE CASCADE,
    -- Trimmed + lowercased, like users.email.
    email      text        NOT NULL,
    role       text        NOT NULL CHECK (role IN ('owner', 'admin', 'member', 'accountant')),
    -- sha256 (hex) of the token in the link.
    token_hash text        NOT NULL CONSTRAINT space_invites_token_hash_key UNIQUE,
    invited_by uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    CONSTRAINT space_invites_space_email_key UNIQUE (space_id, email)
);
"#;

const DOWN: &str = "DROP TABLE space_invites;";

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
