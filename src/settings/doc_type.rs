//! Number series keys. The seven issued keys are also the document types
//! (`documents.doc_type`); the `received*` keys only name the series of the
//! received documents of each type.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    Invoice,
    CreditNote,
    DebitNote,
    Proforma,
    AdvanceTaxDoc,
    AdvanceCreditNote,
    Simplified,
    Received,
    ReceivedCreditNote,
    ReceivedDebitNote,
    ReceivedProforma,
    ReceivedAdvanceTaxDoc,
    ReceivedAdvanceCreditNote,
    ReceivedSimplified,
}

/// `documents.direction`.
pub const ISSUED: &str = "issued";
pub const RECEIVED: &str = "received";

impl DocType {
    /// Every series, in display order.
    pub const ALL: [DocType; 14] = [
        DocType::Invoice,
        DocType::CreditNote,
        DocType::DebitNote,
        DocType::Proforma,
        DocType::AdvanceTaxDoc,
        DocType::AdvanceCreditNote,
        DocType::Simplified,
        DocType::Received,
        DocType::ReceivedCreditNote,
        DocType::ReceivedDebitNote,
        DocType::ReceivedProforma,
        DocType::ReceivedAdvanceTaxDoc,
        DocType::ReceivedAdvanceCreditNote,
        DocType::ReceivedSimplified,
    ];

    /// Wire and database value.
    pub fn as_str(self) -> &'static str {
        match self {
            DocType::Invoice => "invoice",
            DocType::CreditNote => "credit_note",
            DocType::DebitNote => "debit_note",
            DocType::Proforma => "proforma",
            DocType::AdvanceTaxDoc => "advance_tax_doc",
            DocType::AdvanceCreditNote => "advance_credit_note",
            DocType::Simplified => "simplified",
            DocType::Received => "received",
            DocType::ReceivedCreditNote => "received_credit_note",
            DocType::ReceivedDebitNote => "received_debit_note",
            DocType::ReceivedProforma => "received_proforma",
            DocType::ReceivedAdvanceTaxDoc => "received_advance_tax_doc",
            DocType::ReceivedAdvanceCreditNote => "received_advance_credit_note",
            DocType::ReceivedSimplified => "received_simplified",
        }
    }

    pub fn parse(s: &str) -> Option<DocType> {
        DocType::ALL.into_iter().find(|d| d.as_str() == s)
    }

    /// A document type (`invoice` … `simplified`), never a series-only key.
    pub fn parse_document(s: &str) -> Option<DocType> {
        DocType::parse(s).filter(|d| d.is_document_type())
    }

    pub fn is_document_type(self) -> bool {
        matches!(
            self,
            DocType::Invoice
                | DocType::CreditNote
                | DocType::DebitNote
                | DocType::Proforma
                | DocType::AdvanceTaxDoc
                | DocType::AdvanceCreditNote
                | DocType::Simplified
        )
    }

    /// A correction bound to the document it corrects (native: its rate,
    /// currency, contact and VAT mode; `correctionReason` required at issue).
    pub fn is_correction(self) -> bool {
        matches!(
            self,
            DocType::CreditNote | DocType::DebitNote | DocType::AdvanceCreditNote
        )
    }

    /// `-1` for the types stored positive that reduce the original.
    pub fn sign(self) -> i8 {
        match self {
            DocType::CreditNote | DocType::AdvanceCreditNote => -1,
            _ => 1,
        }
    }

    /// The series numbering documents of this type in `direction`.
    pub fn series(self, direction: &str) -> DocType {
        match (direction, self) {
            (RECEIVED, DocType::Invoice) => DocType::Received,
            (RECEIVED, DocType::CreditNote) => DocType::ReceivedCreditNote,
            (RECEIVED, DocType::DebitNote) => DocType::ReceivedDebitNote,
            (RECEIVED, DocType::Proforma) => DocType::ReceivedProforma,
            (RECEIVED, DocType::AdvanceTaxDoc) => DocType::ReceivedAdvanceTaxDoc,
            (RECEIVED, DocType::AdvanceCreditNote) => DocType::ReceivedAdvanceCreditNote,
            (RECEIVED, DocType::Simplified) => DocType::ReceivedSimplified,
            _ => self,
        }
    }

    /// `(direction, doc_type)` of the documents a series numbers.
    pub fn numbered(self) -> (&'static str, &'static str) {
        match self {
            DocType::Received => (RECEIVED, "invoice"),
            DocType::ReceivedCreditNote => (RECEIVED, "credit_note"),
            DocType::ReceivedDebitNote => (RECEIVED, "debit_note"),
            DocType::ReceivedProforma => (RECEIVED, "proforma"),
            DocType::ReceivedAdvanceTaxDoc => (RECEIVED, "advance_tax_doc"),
            DocType::ReceivedAdvanceCreditNote => (RECEIVED, "advance_credit_note"),
            DocType::ReceivedSimplified => (RECEIVED, "simplified"),
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
        assert_eq!(
            DocType::AdvanceCreditNote.series(RECEIVED),
            DocType::ReceivedAdvanceCreditNote
        );
        assert_eq!(
            DocType::ALL.iter().filter(|d| d.is_document_type()).count(),
            7
        );
    }

    #[test]
    fn sign_and_corrections() {
        let neg: Vec<_> = DocType::ALL.into_iter().filter(|d| d.sign() < 0).collect();
        assert_eq!(neg, [DocType::CreditNote, DocType::AdvanceCreditNote]);
        let corr: Vec<_> = DocType::ALL
            .into_iter()
            .filter(|d| d.is_correction())
            .collect();
        assert_eq!(
            corr,
            [
                DocType::CreditNote,
                DocType::DebitNote,
                DocType::AdvanceCreditNote
            ]
        );
    }
}
