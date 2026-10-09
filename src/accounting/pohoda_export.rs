//! The Pohoda file of the accountant export: every document mapped inside
//! the snapshot before anything is sent, so the accountant never gets a
//! file with documents silently missing.

use anyhow::Context as _;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseTransaction, EntityTrait};
use uuid::Uuid;

use super::pohoda::{self, Ctx};
use super::pohoda_summary::third_rate;
use crate::csvio::export_load::{CHUNK, Loaded, rollback};
use crate::error::AppError;
use crate::settings::entity::company;
use crate::settings::repo::vat_rates;

/// Longest `detail` of an `unexportable` error, in characters.
const MAX_DETAIL: usize = 2000;

const LABEL: &str = "accountant pohoda";

async fn ctx<C: ConnectionTrait>(db: &C, from: NaiveDate, to: NaiveDate) -> Result<Ctx, AppError> {
    let settings = super::repo::get(db).await?;
    let company = company::Entity::find_by_id(company::SINGLETON_ID)
        .one(db)
        .await?
        .context("company singleton row missing (migration seeds it)")?;
    let rates: Vec<(Decimal, bool)> = vat_rates::list(db)
        .await?
        .into_iter()
        .map(|r| (r.rate, r.active))
        .collect();
    Ok(Ctx {
        ico: settings.pohoda.ico.clone().or(company.ico),
        settings,
        third: third_rate(&rates),
        from,
        to,
    })
}

/// 422 `documents: unexportable`, `detail` = `"<number>: <reason>; …"`
/// cut to [`MAX_DETAIL`] with the count of the documents left out.
pub fn unexportable(failures: &[(String, String)]) -> AppError {
    let mut detail = String::new();
    for (i, (doc, reason)) in failures.iter().enumerate() {
        let part = format!("{}{doc}: {reason}", if i == 0 { "" } else { "; " });
        if detail.chars().count() + part.chars().count() > MAX_DETAIL {
            detail.push_str(&format!("; … (+{} more)", failures.len() - i));
            break;
        }
        detail.push_str(&part);
    }
    AppError::field_detail("documents", "unexportable", detail)
}

async fn build(txn: &DatabaseTransaction, ids: &[Uuid], ctx: &Ctx) -> Result<String, AppError> {
    let mut xml = pohoda::head(ctx);
    let mut failures = Vec::new();
    let none = Default::default();
    for chunk in ids.chunks(CHUNK) {
        let loaded = Loaded::load(txn, chunk, false).await?;
        for s in loaded.sources(&none) {
            match pohoda::item(&s, ctx) {
                Ok(item) => xml.push_str(&item),
                Err(e) => {
                    let name = pohoda::shown_number(s.doc)
                        .map_or_else(|| s.doc.id.to_string(), str::to_string);
                    tracing::warn!(export = LABEL, document = %s.doc.id, error = %format!("{e:#}"), "document not exportable");
                    failures.push((name, format!("{e:#}")));
                }
            }
        }
    }
    if !failures.is_empty() {
        return Err(unexportable(&failures));
    }
    xml.push_str(pohoda::TAIL);
    Ok(xml)
}

/// The Windows-1250 file of `ids` (already ordered), read in `txn` (ended
/// here). No document → 422 `from: empty` (an empty `dataPack` is not
/// valid); one that cannot be mapped → [`unexportable`].
pub async fn file(
    txn: DatabaseTransaction,
    ids: &[Uuid],
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<u8>, AppError> {
    if ids.is_empty() {
        rollback(txn, LABEL).await;
        return Err(AppError::field("from", "empty"));
    }
    let built = match ctx(&txn, from, to).await {
        Ok(c) => build(&txn, ids, &c).await,
        Err(e) => Err(e),
    };
    match built {
        Ok(xml) => {
            txn.commit().await?;
            Ok(pohoda::encode(&xml))
        }
        Err(e) => {
            rollback(txn, LABEL).await;
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detail(e: AppError) -> (Option<&'static str>, String) {
        match e {
            AppError::ValidationDetail(f, d) => (f.get("documents"), d),
            other => panic!("expected validation detail, got {other:?}"),
        }
    }

    #[test]
    fn unexportable_detail() {
        let f = |n: &str| {
            (
                n.to_string(),
                "VAT rate 10 % maps to no Pohoda rate slot".to_string(),
            )
        };
        let (reason, d) = detail(unexportable(&[f("FV-50"), f("FA-9")]));
        assert_eq!(reason, Some("unexportable"));
        assert_eq!(
            d,
            "FV-50: VAT rate 10 % maps to no Pohoda rate slot; FA-9: VAT rate 10 % maps to no Pohoda rate slot"
        );
        let many: Vec<_> = (0..200).map(|i| f(&format!("N-{i}"))).collect();
        let (_, d) = detail(unexportable(&many));
        assert!(d.chars().count() <= MAX_DETAIL + 20, "{}", d.len());
        assert!(d.starts_with("N-0: ") && d.ends_with("more)"), "{d}");
    }
}
