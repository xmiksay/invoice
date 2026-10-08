//! The archive of issued documents: rendered once, stored, never re-rendered.

use anyhow::Context as _;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement, TransactionTrait};
use uuid::Uuid;

use super::{PdfService, payload, source, storage};
use crate::document::entity::document;
use crate::document::line::Status;
use crate::document::repo::query;
use crate::error::AppError;

/// Render the document as stored in `db` (draft → live data and watermark,
/// cancelled → "STORNO" watermark; neither gets a QR code).
pub async fn render<C: ConnectionTrait>(
    db: &C,
    pdf: &PdfService,
    id: Uuid,
) -> Result<(document::Model, Vec<u8>), AppError> {
    let src = source::load(db, id).await?;
    let data = payload::build(&src.input()?)?;
    let bytes = pdf.render(data).await?;
    Ok((src.row, bytes))
}

/// Record the archive on the row unless one is already recorded, then write
/// the file. Runs inside `txn`, so the file exists before anyone can see the
/// path; `None` when another writer won.
async fn store<C: ConnectionTrait>(
    txn: &C,
    pdf: &PdfService,
    row: &document::Model,
    bytes: &[u8],
) -> Result<Option<String>, AppError> {
    let year = row
        .number_year
        .with_context(|| format!("issued document {} has no number year", row.id))?;
    let rel = storage::relative_path(year, row.id);
    let updated = txn
        .execute(Statement::from_sql_and_values(
            txn.get_database_backend(),
            "UPDATE documents SET pdf_path = $2, pdf_sha256 = $3, pdf_rendered_at = now() \
             WHERE id = $1 AND pdf_path IS NULL",
            [
                row.id.into(),
                rel.clone().into(),
                storage::sha256_hex(bytes).into(),
            ],
        ))
        .await?;
    if updated.rows_affected() == 0 {
        return Ok(None);
    }
    pdf.write(&rel, bytes.to_vec()).await?;
    Ok(Some(rel))
}

/// Issue: render the just-issued document (row locked by the caller) and
/// archive it in the issue transaction. Returns the written path so the
/// caller can remove it if the commit fails.
pub async fn archive_in<C: ConnectionTrait>(
    txn: &C,
    pdf: &PdfService,
    id: Uuid,
) -> Result<Option<String>, AppError> {
    let (row, bytes) = render(txn, pdf, id).await?;
    store(txn, pdf, &row, &bytes).await
}

/// Render and archive an issued document that has no archive yet (a DDPP
/// after its payment, or on its first download). Concurrent callers race on
/// `WHERE pdf_path IS NULL`; the loser serves the winner's stored file, so
/// every download of a document returns the same bytes.
pub async fn archive_missing(
    db: &DatabaseConnection,
    pdf: &PdfService,
    id: Uuid,
) -> Result<Vec<u8>, AppError> {
    let (row, bytes) = render(db, pdf, id).await?;
    let txn = db.begin().await?;
    let stored = match store(&txn, pdf, &row, &bytes).await {
        Ok(s) => s,
        Err(e) => {
            if let Err(r) = txn.rollback().await {
                tracing::warn!(error = %r, "rollback PDF archive");
            }
            return Err(e);
        }
    };
    if let Err(e) = txn.commit().await {
        if let Some(rel) = &stored {
            pdf.remove(rel).await;
        }
        return Err(e.into());
    }
    if stored.is_some() {
        return Ok(bytes);
    }
    // The losing UPDATE waited for the winner's commit, so its path is set.
    let row = query::find(db, id).await?;
    let rel = row
        .pdf_path
        .with_context(|| format!("document {id} lost the archive race but has no archive"))?;
    Ok(pdf.read(&rel).await?)
}

/// After an auto-issued DDPP's payment committed: archive it now if mdcast
/// is up; otherwise its first download does.
pub async fn try_archive_ddpp(db: &DatabaseConnection, pdf: &PdfService, id: Uuid) {
    if let Err(e) = archive_missing(db, pdf, id).await {
        tracing::warn!(document = %id, error = %e, "DDPP PDF not archived yet");
    }
}

/// [`try_archive_ddpp`] in the background, so mdcast never delays the
/// payment response.
pub fn spawn_archive_ddpp(db: DatabaseConnection, pdf: PdfService, id: Uuid) {
    tokio::spawn(async move { try_archive_ddpp(&db, &pdf, id).await });
}

/// The PDF for `GET /api/documents/{id}/pdf`: a draft is rendered live and
/// never stored; an issued or cancelled document serves its archive.
pub async fn document_pdf(
    db: &DatabaseConnection,
    pdf: &PdfService,
    id: Uuid,
) -> Result<(document::Model, Vec<u8>), AppError> {
    let row = query::find(db, id).await?;
    if row.status == Status::Draft.as_str() {
        return render(db, pdf, id).await;
    }
    let bytes = match &row.pdf_path {
        Some(rel) => pdf.read(rel).await?,
        None => archive_missing(db, pdf, id).await?,
    };
    Ok((row, bytes))
}
