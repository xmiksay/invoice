//! The stored draft a `PUT` replaces, and the rules it imposes on the request.

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::document::line::VatMode;
use crate::error::FieldErrors;
use crate::settings::doc_type::DocType;
use crate::validation::Check;

/// The draft being updated, as far as validation cares.
#[derive(Debug, Clone)]
pub struct Existing {
    pub id: Uuid,
    pub doc_type: DocType,
    pub imported: bool,
    pub related_document_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub currency: String,
    pub locale: String,
    pub vat_mode: VatMode,
    /// A correction keeps its original's rate whatever the request says.
    pub exchange_rate: Option<Decimal>,
}

impl Existing {
    /// A correction stays bound to its original's contact, currency and VAT
    /// mode (copied at creation): anything else is `invalid`.
    pub fn check_bound(
        &self,
        currency: &str,
        contact_id: Option<Uuid>,
        vat_mode: VatMode,
        e: &mut FieldErrors,
    ) {
        if currency != self.currency {
            e.add("currency", "invalid");
        }
        if contact_id != self.contact_id {
            e.add("contactId", "invalid");
        }
        if vat_mode != self.vat_mode {
            e.add("vatMode", "invalid");
        }
    }
}

/// The document type a request may set: on create an invoice, proforma or
/// simplified document (an imported document: any of the seven); on update
/// only the draft's own type.
pub fn doc_type(
    requested: Option<&str>,
    existing: Option<&Existing>,
    imported: bool,
) -> Check<DocType> {
    let parsed = requested.map(|t| DocType::parse_document(t.trim()).ok_or("invalid"));
    match (existing, parsed) {
        (_, Some(Err(r))) => Err(r),
        (Some(x), None) => Ok(x.doc_type),
        (Some(x), Some(Ok(t))) if t == x.doc_type => Ok(t),
        (None, None) => Ok(DocType::Invoice),
        (None, Some(Ok(t))) if imported => Ok(t),
        (None, Some(Ok(t @ (DocType::Invoice | DocType::Proforma | DocType::Simplified)))) => Ok(t),
        _ => Err("invalid"),
    }
}
