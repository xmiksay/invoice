//! Database lookups of the import: existing contact, duplicate, related
//! original. `analyze` runs them for the preview and for the selected
//! entries of a confirm; `store` runs them again inside each entry's
//! transaction, the duplicate check under an advisory lock on the
//! document's identity.

use sea_orm::sea_query::{Expr, SimpleExpr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use uuid::Uuid;

use super::model::Party;
use super::plan::Plan;
use crate::contact::entity::contact;
use crate::document::entity::document::{Column, Entity};
use crate::document::handlers::meta::related_types;
use crate::document::line::Status;
use crate::error::AppError;
use crate::settings::doc_type::{DocType, ISSUED};

/// Identity of a party: its IČO, else its name.
pub fn party_key(p: &Party) -> String {
    match &p.ico {
        Some(ico) => format!("ico:{ico}"),
        None => format!("name:{}", p.name),
    }
}

/// The contact with the party's IČO; without an IČO, one with no IČO and
/// exactly the party's name.
pub async fn contact<C: ConnectionTrait>(
    db: &C,
    p: &Party,
) -> Result<Option<contact::Model>, AppError> {
    let q = contact::Entity::find();
    let q = match &p.ico {
        Some(ico) => q.filter(contact::Column::Ico.eq(ico.as_str())),
        None => q
            .filter(contact::Column::Ico.is_null())
            .filter(contact::Column::Name.eq(p.name.as_str())),
    };
    Ok(q.one(db).await?)
}

/// Received documents of the same supplier (snapshot): by IČO, else by name
/// among those without one.
fn same_supplier(p: &Party) -> SimpleExpr {
    match &p.ico {
        Some(ico) => Expr::cust_with_values("supplier_snapshot->>'ico' = $1", [ico.clone()]),
        None => Expr::cust_with_values(
            "supplier_snapshot->>'ico' IS NULL AND supplier_snapshot->>'name' = $1",
            [p.name.clone()],
        ),
    }
}

/// Issued: same type and number (drafts included). Received: same supplier
/// and supplier number.
pub async fn duplicate<C: ConnectionTrait>(db: &C, plan: &Plan) -> Result<bool, AppError> {
    let q = Entity::find().filter(Column::Direction.eq(plan.direction));
    let q = if plan.direction == ISSUED {
        q.filter(Column::DocType.eq(plan.doc_type.as_str()))
            .filter(Column::Number.eq(plan.number.as_str()))
    } else {
        q.filter(Column::SupplierNumber.eq(plan.number.as_str()))
            .filter(same_supplier(&plan.supplier))
    };
    Ok(q.select_only()
        .column(Column::Id)
        .into_tuple::<Uuid>()
        .one(db)
        .await?
        .is_some())
}

/// Types `doc_type` may link to (the 1f-a table).
pub fn targets(doc_type: DocType) -> Vec<&'static str> {
    related_types(doc_type).iter().map(|t| t.as_str()).collect()
}

/// The non-cancelled, non-draft original `OriginalDocumentReference` names.
pub async fn related<C: ConnectionTrait>(db: &C, plan: &Plan) -> Result<Option<Uuid>, AppError> {
    let Some(reference) = &plan.original_ref else {
        return Ok(None);
    };
    let types = targets(plan.doc_type);
    if types.is_empty() {
        return Ok(None);
    }
    let q = Entity::find()
        .filter(Column::Direction.eq(plan.direction))
        .filter(Column::DocType.is_in(types))
        .filter(Column::Status.eq(Status::Issued.as_str()));
    let q = if plan.direction == ISSUED {
        q.filter(Column::Number.eq(reference.as_str()))
    } else {
        q.filter(Column::SupplierNumber.eq(reference.as_str()))
            .filter(same_supplier(&plan.supplier))
    };
    Ok(q.select_only()
        .column(Column::Id)
        .into_tuple::<Uuid>()
        .one(db)
        .await?)
}

/// Batch identity of a document: two entries with the same one are the
/// same document.
pub fn identity(plan: &Plan) -> String {
    if plan.direction == ISSUED {
        format!("issued|{}|{}", plan.doc_type.as_str(), plan.number)
    } else {
        format!("received|{}|{}", party_key(&plan.supplier), plan.number)
    }
}

/// `other` is the original `plan` references.
pub fn is_original_of(plan: &Plan, other: &Plan) -> bool {
    plan.original_ref.as_deref() == Some(other.number.as_str())
        && plan.direction == other.direction
        && targets(plan.doc_type).contains(&other.doc_type.as_str())
        && (plan.direction == ISSUED || party_key(&plan.supplier) == party_key(&other.supplier))
}

/// Import order: originals before the documents linking to them (every
/// link-table target has a lower rank).
pub fn rank(t: DocType) -> u8 {
    match t {
        DocType::Proforma => 0,
        DocType::CreditNote | DocType::DebitNote | DocType::AdvanceCreditNote => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_targets_rank_lower() {
        for t in DocType::ALL.into_iter().filter(|t| t.is_document_type()) {
            for target in related_types(t) {
                assert!(rank(*target) < rank(t), "{t:?} → {target:?}");
            }
        }
    }

    #[test]
    fn party_keys() {
        let mut p = Party {
            name: "A".into(),
            ico: Some("12345679".into()),
            ..Default::default()
        };
        assert_eq!(party_key(&p), "ico:12345679");
        p.ico = None;
        assert_eq!(party_key(&p), "name:A");
    }
}
