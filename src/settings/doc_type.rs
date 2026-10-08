//! Number series keys. The first four are also the document types
//! (`documents.doc_type`); the `received*` keys only name the series of the
//! received documents of each type.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    Invoice,
    CreditNote,
    Proforma,
    AdvanceTaxDoc,
    Received,
    ReceivedCreditNote,
    ReceivedProforma,
    ReceivedAdvanceTaxDoc,
}

/// `documents.direction`.
pub const ISSUED: &str = "issued";
pub const RECEIVED: &str = "received";

impl DocType {
    /// Every series, in display order.
    pub const ALL: [DocType; 8] = [
        DocType::Invoice,
        DocType::CreditNote,
        DocType::Proforma,
        DocType::AdvanceTaxDoc,
        DocType::Received,
        DocType::ReceivedCreditNote,
        DocType::ReceivedProforma,
        DocType::ReceivedAdvanceTaxDoc,
    ];

    /// Wire and database value.
    pub fn as_str(self) -> &'static str {
        match self {
            DocType::Invoice => "invoice",
            DocType::CreditNote => "credit_note",
            DocType::Proforma => "proforma",
            DocType::AdvanceTaxDoc => "advance_tax_doc",
            DocType::Received => "received",
            DocType::ReceivedCreditNote => "received_credit_note",
            DocType::ReceivedProforma => "received_proforma",
            DocType::ReceivedAdvanceTaxDoc => "received_advance_tax_doc",
        }
    }

    pub fn parse(s: &str) -> Option<DocType> {
        DocType::ALL.into_iter().find(|d| d.as_str() == s)
    }

    /// A document type (`invoice` … `advance_tax_doc`), never a series-only key.
    pub fn parse_document(s: &str) -> Option<DocType> {
        DocType::parse(s).filter(|d| d.is_document_type())
    }

    pub fn is_document_type(self) -> bool {
        matches!(
            self,
            DocType::Invoice | DocType::CreditNote | DocType::Proforma | DocType::AdvanceTaxDoc
        )
    }

    /// The series numbering documents of this type in `direction`.
    pub fn series(self, direction: &str) -> DocType {
        match (direction, self) {
            (RECEIVED, DocType::Invoice) => DocType::Received,
            (RECEIVED, DocType::CreditNote) => DocType::ReceivedCreditNote,
            (RECEIVED, DocType::Proforma) => DocType::ReceivedProforma,
            (RECEIVED, DocType::AdvanceTaxDoc) => DocType::ReceivedAdvanceTaxDoc,
            _ => self,
        }
    }

    /// `(direction, doc_type)` of the documents a series numbers.
    pub fn numbered(self) -> (&'static str, &'static str) {
        match self {
            DocType::Received => (RECEIVED, "invoice"),
            DocType::ReceivedCreditNote => (RECEIVED, "credit_note"),
            DocType::ReceivedProforma => (RECEIVED, "proforma"),
            DocType::ReceivedAdvanceTaxDoc => (RECEIVED, "advance_tax_doc"),
            other => (ISSUED, other.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_str_and_serde() {
        for d in DocType::ALL {
            assert_eq!(DocType::parse(d.as_str()), Some(d));
            assert_eq!(
                serde_json::to_value(d).expect("serialize"),
                serde_json::Value::String(d.as_str().into())
            );
        }
        assert_eq!(DocType::parse("order"), None);
        assert_eq!(DocType::parse_document("received"), None);
        assert_eq!(DocType::parse_document("proforma"), Some(DocType::Proforma));
    }

    #[test]
    fn series_and_numbered_are_inverse() {
        for d in DocType::ALL.into_iter().filter(|d| d.is_document_type()) {
            for dir in [ISSUED, RECEIVED] {
                let s = d.series(dir);
                assert_eq!(s.numbered(), (dir, d.as_str()));
            }
        }
        assert_eq!(
            DocType::Proforma.series(RECEIVED),
            DocType::ReceivedProforma
        );
    }
}
