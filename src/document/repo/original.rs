//! The uploaded original PDF of a received or imported document.

use chrono::Datelike;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DatabaseTransaction, Set, TransactionTrait};
use uuid::Uuid;

use super::query;
use crate::document::entity::document;
use crate::error::AppError;
use crate::pdf::{PdfService, storage};
use crate::settings::doc_type::RECEIVED;

/// `documents/{year}/{id}-original-{sha8}.pdf`: content-addressed, so a
/// replacement never overwrites the committed file.
pub fn relative_path(year: i32, id: Uuid, sha256: &str) -> String {
    let short = sha256.get(..8).unwrap_or(sha256);
    format!("documents/{year}/{id}-original-{short}.pdf")
}

/// Received documents and imported issued ones (any status); a native
/// issued document's PDF is its rendered archive.
fn allowed(doc: &document::Model) -> Result<(), AppError> {
    if doc.direction == RECEIVED || doc.imported {
        Ok(())
    } else {
        Err(AppError::InvalidState)
    }
}

/// Store `bytes` (already checked to be a PDF) as the document's original,
/// replacing any previous one. The new file gets its own name and is written
/// inside the transaction; the old file is removed only after the commit, the
/// new one if the commit fails.
pub async fn put(
    db: &DatabaseConnection,
    pdf: &PdfService,
    id: Uuid,
    bytes: Vec<u8>,
) -> Result<(), AppError> {
    let txn = db.begin().await?;
    let (rel, previous) = match put_in(&txn, pdf, id, bytes).await {
        Ok(paths) => paths,
        Err(e) => {
            if let Err(r) = txn.rollback().await {
                tracing::warn!(error = %r, "rollback original upload");
            }
            return Err(e);
        }
    };
    // Same content → same name: then that file backs the stored row whatever
    // happens, so it is never removed.
    let same = previous.as_deref() == Some(rel.as_str());
    if let Err(e) = txn.commit().await {
        if !same {
            pdf.remove(&rel).await;
        }
        return Err(e.into());
    }
    if let Some(old) = previous.filter(|_| !same) {
        pdf.remove(&old).await;
    }
    Ok(())
}

/// Returns the new path and the previously stored one.
async fn put_in(
    txn: &DatabaseTransaction,
    pdf: &PdfService,
    id: Uuid,
    bytes: Vec<u8>,
) -> Result<(String, Option<String>), AppError> {
    let doc = query::lock(txn, id).await?;
    allowed(&doc)?;
    let sha256 = storage::sha256_hex(&bytes);
    let year = doc.number_year.unwrap_or(doc.issue_date.year());
    let rel = relative_path(year, id, &sha256);
    let previous = doc.original_path.clone();
    let size = i64::try_from(bytes.len()).map_err(|_| AppError::TooLarge)?;
    let now = chrono::Utc::now();
    let mut row: document::ActiveModel = doc.into();
    row.original_path = Set(Some(rel.clone()));
    row.original_sha256 = Set(Some(sha256));
    row.original_size = Set(Some(size));
    row.original_uploaded_at = Set(Some(now.into()));
    row.updated_at = Set(now.into());
    row.update(txn).await?;
    pdf.write(&rel, bytes).await?;
    Ok((rel, previous))
}

/// Idempotent; the file is removed after the commit (best effort).
pub async fn delete(db: &DatabaseConnection, pdf: &PdfService, id: Uuid) -> Result<(), AppError> {
    let removed = db
        .transaction(|txn| {
            Box::pin(async move {
                let doc = query::lock(txn, id).await?;
                allowed(&doc)?;
                let rel = doc.original_path.clone();
                let mut row: document::ActiveModel = doc.into();
                row.original_path = Set(None);
                row.original_sha256 = Set(None);
                row.original_size = Set(None);
                row.original_uploaded_at = Set(None);
                row.updated_at = Set(chrono::Utc::now().into());
                row.update(txn).await?;
                Ok::<_, AppError>(rel)
            })
        })
        .await?;
    if let Some(rel) = removed {
        pdf.remove(&rel).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_path() {
        assert_eq!(
            relative_path(2026, Uuid::nil(), "ba7816bf8f01cfea"),
            "documents/2026/00000000-0000-0000-0000-000000000000-original-ba7816bf.pdf"
        );
    }
}
