//! Loading the documents `advance` lines reference, and the in-transaction
//! re-check that a deducted document is still available.

use std::collections::{HashMap, HashSet};

use anyhow::Context as _;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use super::view;
use crate::document::advance::{self, AdvanceSource};
use crate::document::entity::{document, document_line, vat_recap};
use crate::document::line::{AdvanceRow, LineData, Status};
use crate::error::{AppError, FieldErrors};

/// `(referenced document, referencing invoice)` for every non-cancelled
/// invoice with an advance line on one of `ids`.
async fn referencing<C: ConnectionTrait>(
    db: &C,
    ids: &[Uuid],
) -> Result<Vec<(Uuid, Uuid)>, AppError> {
    let lines = document_line::Entity::find()
        .filter(document_line::Column::AdvanceDocumentId.is_in(ids.to_vec()))
        .all(db)
        .await?;
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let holders: Vec<Uuid> = lines.iter().map(|l| l.document_id).collect();
    let live: HashSet<Uuid> = document::Entity::find()
        .select_only()
        .column(document::Column::Id)
        .filter(document::Column::Id.is_in(holders))
        .filter(document::Column::DocType.eq("invoice"))
        .filter(document::Column::Status.ne(Status::Cancelled.as_str()))
        .into_tuple::<Uuid>()
        .all(db)
        .await?
        .into_iter()
        .collect();
    Ok(lines
        .into_iter()
        .filter(|l| live.contains(&l.document_id))
        .filter_map(|l| l.advance_document_id.map(|a| (a, l.document_id)))
        .collect())
}

/// The documents named by `ids`, with their recap and current references.
pub async fn load<C: ConnectionTrait>(
    db: &C,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, AdvanceSource>, AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    // Received documents can never be deducted: as if they did not exist.
    let docs = document::Entity::find()
        .filter(document::Column::Id.is_in(ids.to_vec()))
        .filter(document::Column::Direction.eq(crate::settings::doc_type::ISSUED))
        .all(db)
        .await?;
    let recaps = vat_recap::Entity::find()
        .filter(vat_recap::Column::DocumentId.is_in(ids.to_vec()))
        .order_by_desc(vat_recap::Column::VatRate)
        .all(db)
        .await?;
    let refs = referencing(db, ids).await?;
    let mut out = HashMap::with_capacity(docs.len());
    for d in docs {
        let supplier: Option<serde_json::Value> = d.supplier_snapshot.clone();
        let vat_payer = supplier
            .as_ref()
            .and_then(|s| s.get("vatPayer"))
            .and_then(serde_json::Value::as_bool);
        let source = AdvanceSource {
            id: d.id,
            status: view::status(&d)?,
            issues_ddpp: advance::issues_ddpp(vat_payer, &d.vat_mode),
            recap: recaps
                .iter()
                .filter(|r| r.document_id == d.id)
                .map(|r| AdvanceRow {
                    vat_rate: r.vat_rate.normalize(),
                    base: r.base,
                    vat: r.vat,
                    base_czk: r.base_czk,
                    vat_czk: r.vat_czk,
                })
                .collect(),
            referenced_by: refs
                .iter()
                .filter(|(a, _)| *a == d.id)
                .map(|(_, holder)| *holder)
                .collect(),
            doc_type: d.doc_type,
            contact_id: d.contact_id,
            currency: d.currency,
            number: d.number,
            paid: d.paid,
        };
        out.insert(source.id, source);
    }
    Ok(out)
}

/// Inside the writing transaction: lock every referenced document (in id
/// order, so concurrent writers queue instead of deadlocking) and re-check
/// what may have changed since validation — still issued, not taken by
/// another invoice meanwhile, and (non-payer form) the paid amount unchanged.
pub async fn lock_and_recheck<C: ConnectionTrait>(
    txn: &C,
    document_id: Uuid,
    lines: &[LineData],
) -> Result<(), AppError> {
    let mut ids = advance::referenced_ids(lines);
    if ids.is_empty() {
        return Ok(());
    }
    ids.sort();
    ids.dedup();
    let locked: HashMap<Uuid, document::Model> = document::Entity::find()
        .filter(document::Column::Id.is_in(ids.clone()))
        .order_by_asc(document::Column::Id)
        .lock_exclusive()
        .all(txn)
        .await?
        .into_iter()
        .map(|d| (d.id, d))
        .collect();
    let refs = referencing(txn, &ids).await?;
    let mut e = FieldErrors::new();
    for (i, line) in lines.iter().enumerate() {
        let LineData::Advance(a) = line else { continue };
        let field = format!("lines.{i}.advanceDocumentId");
        let doc = locked
            .get(&a.document_id)
            .context("advance document vanished")?;
        if view::status(doc)? != Status::Issued {
            e.add(&field, "invalid");
        } else if refs
            .iter()
            .any(|(adv, holder)| *adv == a.document_id && *holder != document_id)
        {
            e.add(&field, "duplicate");
        } else if doc.doc_type == "proforma" && a.recap.first().map(|r| r.base) != Some(doc.paid) {
            return Err(AppError::Conflict("proforma payments changed".into()));
        }
    }
    e.into_result()
}
