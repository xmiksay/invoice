//! Settings → Accounting: per program, per direction × exported document
//! type, the optional codes the export writes. Pure (shape + validation).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{AppError, FieldErrors};
use crate::settings::doc_type::{DocType, ISSUED, RECEIVED};
use crate::validation as v;

/// The document types the accounting exports hold (no proforma).
pub const EXPORTED: [DocType; 6] = [
    DocType::Invoice,
    DocType::CreditNote,
    DocType::DebitNote,
    DocType::AdvanceTaxDoc,
    DocType::AdvanceCreditNote,
    DocType::Simplified,
];

pub const DIRECTIONS: [&str; 2] = [ISSUED, RECEIVED];

/// Longest code: Pohoda's `typ:idsType` (`maxLength` 19).
pub const MAX_CODE: usize = 19;

/// Request and response share the shape. The Money S3 section comes in 3c.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AccountingSettings {
    #[serde(default)]
    pub pohoda: PohodaSettings,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PohodaSettings {
    /// `dataPack/@ico` (the accounting unit); null → the company IČO.
    #[serde(default)]
    pub ico: Option<String>,
    /// Every direction × type row (GET); PUT may send any subset.
    #[serde(default)]
    pub codes: Vec<CodeRow>,
}

/// The codes of one direction × document type; null = not written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CodeRow {
    /// `issued` | `received`.
    pub direction: String,
    /// `invoice` | `credit_note` | `debit_note` | `advance_tax_doc` |
    /// `advance_credit_note` | `simplified`.
    pub doc_type: String,
    /// Předkontace (`accounting/ids`).
    #[serde(default)]
    pub accounting: Option<String>,
    /// Členění DPH (`classificationVAT/ids`).
    #[serde(default)]
    pub classification_vat: Option<String>,
    /// Received only: členění DPH of a document without the VAT deduction;
    /// null → `classificationVATType` `nonSubsume`, never the code above.
    #[serde(default)]
    pub classification_vat_non_deductible: Option<String>,
    /// Číselná řada (`number/ids`).
    #[serde(default)]
    pub number_series: Option<String>,
}

impl CodeRow {
    fn empty(direction: &str, doc_type: DocType) -> Self {
        Self {
            direction: direction.into(),
            doc_type: doc_type.as_str().into(),
            accounting: None,
            classification_vat: None,
            classification_vat_non_deductible: None,
            number_series: None,
        }
    }
}

fn code(s: Option<&str>) -> v::Check<Option<String>> {
    v::opt_text(s, MAX_CODE)
}

fn exported(s: &str) -> Option<DocType> {
    DocType::parse_document(s).filter(|d| EXPORTED.contains(d))
}

impl AccountingSettings {
    /// Normalize (trim, `""` → null), check every row, then [`Self::full`].
    pub fn validate(self) -> Result<Self, AppError> {
        let mut e = FieldErrors::new();
        let ico = e
            .check("pohoda.ico", v::opt_ico(self.pohoda.ico.as_deref()))
            .flatten();
        let mut codes: Vec<CodeRow> = Vec::new();
        for (i, r) in self.pohoda.codes.iter().enumerate() {
            let f = |name: &str| format!("pohoda.codes.{i}.{name}");
            let direction = DIRECTIONS.into_iter().find(|d| *d == r.direction.trim());
            let direction = e.check(&f("direction"), direction.ok_or("invalid"));
            let doc_type = e.check(&f("docType"), exported(r.doc_type.trim()).ok_or("invalid"));
            let accounting = e.check(&f("accounting"), code(r.accounting.as_deref()));
            let classification_vat = e.check(
                &f("classificationVat"),
                code(r.classification_vat.as_deref()),
            );
            let non_deductible = e.check(
                &f("classificationVatNonDeductible"),
                code(r.classification_vat_non_deductible.as_deref()).and_then(|c| {
                    match (c.is_some(), direction) {
                        (true, Some(ISSUED)) => Err("invalid"),
                        _ => Ok(c),
                    }
                }),
            );
            let number_series = e.check(&f("numberSeries"), code(r.number_series.as_deref()));
            let (Some(direction), Some(doc_type)) = (direction, doc_type) else {
                continue;
            };
            if codes
                .iter()
                .any(|c| c.direction == direction && c.doc_type == doc_type.as_str())
            {
                e.add(&f("docType"), "duplicate");
                continue;
            }
            codes.push(CodeRow {
                accounting: accounting.flatten(),
                classification_vat: classification_vat.flatten(),
                classification_vat_non_deductible: non_deductible.flatten(),
                number_series: number_series.flatten(),
                ..CodeRow::empty(direction, doc_type)
            });
        }
        e.into_result()?;
        Ok(Self {
            pohoda: PohodaSettings { ico, codes },
        }
        .full())
    }

    /// Every direction × type row, in order (issued first, [`EXPORTED`]
    /// order); missing rows all null, unknown stored rows dropped.
    pub fn full(self) -> Self {
        let codes = DIRECTIONS
            .into_iter()
            .flat_map(|d| EXPORTED.into_iter().map(move |t| (d, t)))
            .map(|(d, t)| {
                self.pohoda
                    .codes
                    .iter()
                    .find(|c| c.direction == d && c.doc_type == t.as_str())
                    .cloned()
                    .unwrap_or_else(|| CodeRow::empty(d, t))
            })
            .collect();
        Self {
            pohoda: PohodaSettings {
                ico: self.pohoda.ico,
                codes,
            },
        }
    }

    /// The Pohoda codes of `direction` × `doc_type`.
    pub fn pohoda_codes(&self, direction: &str, doc_type: DocType) -> Option<&CodeRow> {
        self.pohoda
            .codes
            .iter()
            .find(|c| c.direction == direction && c.doc_type == doc_type.as_str())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn parse(v: serde_json::Value) -> AccountingSettings {
        serde_json::from_value(v).expect("shape")
    }

    fn errors(v: serde_json::Value) -> FieldErrors {
        match parse(v).validate() {
            Err(AppError::Validation(e)) => e,
            other => panic!("expected validation, got {other:?}"),
        }
    }

    #[test]
    fn full_lists_every_row() {
        let s = AccountingSettings::default().full();
        assert_eq!(s.pohoda.codes.len(), 12);
        assert_eq!(s.pohoda.codes[0], CodeRow::empty(ISSUED, DocType::Invoice));
        assert_eq!(
            s.pohoda.codes[11],
            CodeRow::empty(RECEIVED, DocType::Simplified)
        );
        assert_eq!(
            serde_json::to_value(&s).expect("json")["pohoda"]["ico"],
            json!(null)
        );
    }

    #[test]
    fn normalizes() {
        let s = parse(json!({ "pohoda": { "ico": " 4444 4443 ", "codes": [
            { "direction": "received", "docType": "credit_note", "accounting": " 3Pdob ",
              "classificationVatNonDeductible": " PN ",
              "classificationVat": "", "numberSeries": null },
            { "direction": " issued", "docType": "invoice", "numberSeries": "FV" },
        ]}}))
        .validate()
        .expect("valid");
        assert_eq!(s.pohoda.ico.as_deref(), Some("44444443"));
        assert_eq!(s.pohoda.codes.len(), 12);
        let fv = s.pohoda_codes(ISSUED, DocType::Invoice).expect("row");
        assert_eq!(fv.number_series.as_deref(), Some("FV"));
        let dob = s.pohoda_codes(RECEIVED, DocType::CreditNote).expect("row");
        assert_eq!(dob.accounting.as_deref(), Some("3Pdob"));
        assert_eq!(dob.classification_vat, None);
        assert_eq!(dob.classification_vat_non_deductible.as_deref(), Some("PN"));
        assert_eq!(s.pohoda_codes(ISSUED, DocType::Proforma), None);
    }

    #[test]
    fn rejects() {
        let e = errors(json!({ "pohoda": { "ico": "12345678", "codes": [
            { "direction": "both", "docType": "invoice" },
            { "direction": "issued", "docType": "proforma" },
            { "direction": "issued", "docType": "invoice", "accounting": "x".repeat(20) },
            { "direction": "issued", "docType": "invoice" },
            { "direction": "issued", "docType": "debit_note", "classificationVatNonDeductible": "PN" },
        ]}}));
        assert_eq!(e.get("pohoda.ico"), Some("invalid_ico"));
        assert_eq!(
            e.get("pohoda.codes.4.classificationVatNonDeductible"),
            Some("invalid"),
            "received only"
        );
        assert_eq!(e.get("pohoda.codes.0.direction"), Some("invalid"));
        assert_eq!(e.get("pohoda.codes.1.docType"), Some("invalid"));
        assert_eq!(e.get("pohoda.codes.2.accounting"), Some("too_long"));
        assert_eq!(e.get("pohoda.codes.3.docType"), Some("duplicate"));
        let ok = json!({ "pohoda": { "codes": [
            { "direction": "issued", "docType": "invoice", "accounting": "x".repeat(19) }]}});
        assert!(parse(ok).validate().is_ok());
    }
}
