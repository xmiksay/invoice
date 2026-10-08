//! Document types that own a number series.

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
}

impl DocType {
    /// Every doc type, in display order.
    pub const ALL: [DocType; 5] = [
        DocType::Invoice,
        DocType::CreditNote,
        DocType::Proforma,
        DocType::AdvanceTaxDoc,
        DocType::Received,
    ];

    /// Wire and database value.
    pub fn as_str(self) -> &'static str {
        match self {
            DocType::Invoice => "invoice",
            DocType::CreditNote => "credit_note",
            DocType::Proforma => "proforma",
            DocType::AdvanceTaxDoc => "advance_tax_doc",
            DocType::Received => "received",
        }
    }

    pub fn parse(s: &str) -> Option<DocType> {
        DocType::ALL.into_iter().find(|d| d.as_str() == s)
    }
}

#[cfg(test)]
mod tests {
    use super::DocType;

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
    }
}
