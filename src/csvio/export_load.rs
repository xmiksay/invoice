//! The export's database side: the CSV rate columns, the documents of a
//! chunk with what their rows need ([`Loaded`], shared with the Pohoda
//! export), and the streamed CSV body. Everything runs in the caller's
//! snapshot transaction.

use std::collections::HashMap;

use axum::body::Body;
use futures_util::{StreamExt as _, TryStreamExt as _, stream};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, QueryFilter, QuerySelect,
};
use uuid::Uuid;

use super::export_row::{Source, out_row};
use super::write;
use crate::document::entity::{document, payment, vat_recap};
use crate::error::AppError;
use crate::settings::repo::{categories, vat_rates};

/// Documents loaded (and written) per query round.
pub const CHUNK: usize = 500;

fn group<T>(rows: Vec<T>, key: impl Fn(&T) -> Uuid) -> HashMap<Uuid, Vec<T>> {
    let mut map: HashMap<Uuid, Vec<T>> = HashMap::new();
    for r in rows {
        map.entry(key(&r)).or_default().push(r);
    }
    map
}

/// Settings → VAT rates plus every rate the documents' recap uses.
pub async fn rates<C: ConnectionTrait>(db: &C, ids: &[Uuid]) -> Result<Vec<Decimal>, AppError> {
    let mut rates: Vec<Decimal> = vat_rates::list(db)
        .await?
        .into_iter()
        .map(|r| r.rate)
        .collect();
    for chunk in ids.chunks(CHUNK * 10) {
        let used: Vec<Decimal> = vat_recap::Entity::find()
            .select_only()
            .column(vat_recap::Column::VatRate)
            .distinct()
            .filter(vat_recap::Column::DocumentId.is_in(chunk.to_vec()))
            .into_tuple()
            .all(db)
            .await?;
        rates.extend(used);
    }
    Ok(rates)
}

async fn by_id<C: ConnectionTrait>(
    db: &C,
    mut ids: Vec<Uuid>,
) -> Result<HashMap<Uuid, document::Model>, AppError> {
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(document::Entity::find()
        .filter(document::Column::Id.is_in(ids))
        .all(db)
        .await?
        .into_iter()
        .map(|d| (d.id, d))
        .collect())
}

/// One chunk of documents with their recap, originals and (CSV only)
/// payments.
pub struct Loaded {
    ids: Vec<Uuid>,
    docs: HashMap<Uuid, document::Model>,
    recap: HashMap<Uuid, Vec<vat_recap::Model>>,
    payments: HashMap<Uuid, Vec<payment::Model>>,
    parents: HashMap<Uuid, document::Model>,
}

impl Loaded {
    pub async fn load<C: ConnectionTrait>(
        db: &C,
        ids: &[Uuid],
        with_payments: bool,
    ) -> Result<Self, AppError> {
        let docs = by_id(db, ids.to_vec()).await?;
        let recap = group(
            vat_recap::Entity::find()
                .filter(vat_recap::Column::DocumentId.is_in(ids.to_vec()))
                .all(db)
                .await?,
            |r| r.document_id,
        );
        let payments = match with_payments {
            true => group(
                payment::Entity::find()
                    .filter(payment::Column::DocumentId.is_in(ids.to_vec()))
                    .all(db)
                    .await?,
                |p| p.document_id,
            ),
            false => HashMap::new(),
        };
        let parents = by_id(
            db,
            docs.values()
                .filter_map(|d| d.related_document_id)
                .collect(),
        )
        .await?;
        Ok(Self {
            ids: ids.to_vec(),
            docs,
            recap,
            payments,
            parents,
        })
    }

    /// The documents in `ids` order.
    pub fn sources<'a>(
        &'a self,
        categories: &'a HashMap<Uuid, String>,
    ) -> impl Iterator<Item = Source<'a>> {
        self.ids
            .iter()
            .filter_map(|id| self.docs.get(id))
            .map(|doc| Source {
                doc,
                recap: self.recap.get(&doc.id).map_or(&[], Vec::as_slice),
                payments: self.payments.get(&doc.id).map_or(&[], Vec::as_slice),
                parent: doc.related_document_id.and_then(|p| self.parents.get(&p)),
                category: doc
                    .category_id
                    .and_then(|c| categories.get(&c))
                    .map(String::as_str),
            })
    }
}

/// The CSV rows of `ids`, in that order. A document whose row cannot be
/// built (an undecodable snapshot, a foreign amount without a CZK value…)
/// is skipped and logged: the rest of the file stays complete.
async fn chunk<C: ConnectionTrait>(db: &C, ids: &[Uuid], w: &Walk) -> Result<Vec<u8>, AppError> {
    let loaded = Loaded::load(db, ids, true).await?;
    let mut rows = Vec::with_capacity(ids.len());
    for s in loaded.sources(&w.categories) {
        match out_row(&s) {
            Ok(row) => rows.push(row),
            Err(e) => {
                tracing::error!(export = w.label, document = %s.doc.id, error = %format!("{e:#}"), "export skipped a document");
            }
        }
    }
    Ok(write::body(&w.rates, &rows)?)
}

/// What the stream walks: the snapshot (committed after the last chunk),
/// the chunks left, the header's rates.
struct Walk {
    txn: Option<DatabaseTransaction>,
    chunks: std::vec::IntoIter<Vec<Uuid>>,
    rates: Vec<Decimal>,
    categories: HashMap<Uuid, String>,
    /// Which export, for the logs.
    label: &'static str,
}

async fn next(mut w: Walk) -> Result<Option<(Vec<u8>, Walk)>, AppError> {
    let Some(txn) = w.txn.take() else {
        return Ok(None);
    };
    match w.chunks.next() {
        Some(ids) => {
            let bytes = chunk(&txn, &ids, &w).await?;
            w.txn = Some(txn);
            Ok(Some((bytes, w)))
        }
        // Ended explicitly: a dropped transaction would only queue its
        // rollback and keep the snapshot open on a pooled connection.
        None => {
            txn.commit().await?;
            Ok(None)
        }
    }
}

async fn prepare(
    txn: &DatabaseTransaction,
    ids: &[Uuid],
) -> Result<(Vec<Decimal>, Vec<u8>, HashMap<Uuid, String>), AppError> {
    let rates = rates(txn, ids).await?;
    let head = write::head(&rates)?;
    let categories = categories::list(txn)
        .await?
        .into_iter()
        .map(|c| (c.id, c.name))
        .collect();
    Ok((rates, head, categories))
}

/// Ends a read-only snapshot that failed, logging a failed rollback.
pub async fn rollback(txn: DatabaseTransaction, label: &str) {
    if let Err(r) = txn.rollback().await {
        tracing::warn!(export = label, error = %r, "export rollback failed");
    }
}

/// The CSV of `ids` (already ordered), read in `txn`: the header right
/// away, then the rows chunk by chunk. A failure mid-way aborts the body
/// (logged). `label` names the export in the logs.
pub async fn body(
    txn: DatabaseTransaction,
    ids: Vec<Uuid>,
    label: &'static str,
) -> Result<Body, AppError> {
    let (rates, head, categories) = match prepare(&txn, &ids).await {
        Ok(x) => x,
        Err(e) => {
            rollback(txn, label).await;
            return Err(e);
        }
    };
    let walk = Walk {
        txn: Some(txn),
        chunks: ids
            .chunks(CHUNK)
            .map(<[Uuid]>::to_vec)
            .collect::<Vec<_>>()
            .into_iter(),
        rates,
        categories,
        label,
    };
    let rest = stream::try_unfold(walk, next).map_err(move |e| {
        tracing::error!(export = label, error = %e, "export aborted");
        std::io::Error::other("export failed")
    });
    Ok(Body::from_stream(
        stream::once(async move { Ok(head) }).chain(rest),
    ))
}
