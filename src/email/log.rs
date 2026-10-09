//! The e-mail log of a document: one row per send attempt.

use chrono::{DateTime, FixedOffset};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::entity::{self, Column, Entity};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmailLogEntry {
    pub id: Uuid,
    pub created_at: DateTime<FixedOffset>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    /// Attachment filenames.
    pub attachments: Vec<String>,
    pub ok: bool,
    pub error: Option<String>,
    pub message_id: Option<String>,
}

impl From<entity::Model> for EmailLogEntry {
    fn from(m: entity::Model) -> Self {
        Self {
            id: m.id,
            created_at: m.created_at,
            to: m.to_addrs,
            cc: m.cc_addrs,
            bcc: m.bcc_addrs,
            subject: m.subject,
            body: m.body,
            attachments: m.attachments,
            ok: m.ok,
            error: m.error,
            message_id: m.message_id,
        }
    }
}

/// What one attempt sent.
pub struct Attempt {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<String>,
}

/// The log entry of an attempt (`error` = `None` → sent), built before it
/// is stored so a successful send can be reported even if storing fails.
pub fn entry(
    a: Attempt,
    sent_at: DateTime<FixedOffset>,
    error: Option<String>,
    message_id: Option<String>,
) -> EmailLogEntry {
    EmailLogEntry {
        id: Uuid::new_v4(),
        created_at: sent_at,
        to: a.to,
        cc: a.cc,
        bcc: a.bcc,
        subject: a.subject,
        body: a.body,
        attachments: a.attachments,
        ok: error.is_none(),
        error,
        message_id,
    }
}

pub async fn insert(
    db: &DatabaseConnection,
    document_id: Uuid,
    e: &EmailLogEntry,
) -> Result<(), AppError> {
    entity::ActiveModel {
        id: Set(e.id),
        document_id: Set(document_id),
        created_at: Set(e.created_at),
        to_addrs: Set(e.to.clone()),
        cc_addrs: Set(e.cc.clone()),
        bcc_addrs: Set(e.bcc.clone()),
        subject: Set(e.subject.clone()),
        body: Set(e.body.clone()),
        attachments: Set(e.attachments.clone()),
        ok: Set(e.ok),
        error: Set(e.error.clone()),
        message_id: Set(e.message_id.clone()),
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Newest first.
pub async fn list(
    db: &DatabaseConnection,
    document_id: Uuid,
) -> Result<Vec<EmailLogEntry>, AppError> {
    Ok(Entity::find()
        .filter(Column::DocumentId.eq(document_id))
        .order_by_desc(Column::CreatedAt)
        .order_by_desc(Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt() -> Attempt {
        Attempt {
            to: vec!["a@x.cz".into()],
            cc: vec![],
            bcc: vec!["b@x.cz".into()],
            subject: "S".into(),
            body: "B".into(),
            attachments: vec!["1.pdf".into()],
        }
    }

    #[test]
    fn entries_record_the_outcome() {
        let now = chrono::Utc::now().into();
        let ok = entry(attempt(), now, None, Some("<m@x.cz>".into()));
        assert!(ok.ok && ok.error.is_none());
        assert_eq!(ok.message_id.as_deref(), Some("<m@x.cz>"));
        assert_eq!(
            (ok.bcc.as_slice(), ok.attachments.as_slice()),
            (
                ["b@x.cz".to_string()].as_slice(),
                ["1.pdf".to_string()].as_slice()
            )
        );
        let failed = entry(attempt(), now, Some("550 no".into()), None);
        assert!(!failed.ok);
        assert_eq!(failed.error.as_deref(), Some("550 no"));
        assert_ne!(ok.id, failed.id);
    }
}
