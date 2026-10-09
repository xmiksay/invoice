//! Decimal input fields that accept a JSON number as well as a decimal string
//! (AI clients send `1000` for an amount). The number is taken by its shortest
//! textual form (serde_json's), never through f64 arithmetic, and then goes
//! through the usual string validation (`invalid` for exponents, too many
//! decimal places, …).

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer};

/// Schema stand-in for a decimal string field: `"type": ["string", "number"]`.
pub struct DecimalText;

impl JsonSchema for DecimalText {
    fn schema_name() -> Cow<'static, str> {
        "DecimalText".into()
    }

    fn inline_schema() -> bool {
        true
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": ["string", "number"],
            "description": "Decimal, as a string (\"1234.50\") or a JSON number"
        })
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Text {
    Str(String),
    Num(serde_json::Number),
}

impl From<Text> for String {
    fn from(t: Text) -> Self {
        match t {
            Text::Str(s) => s,
            Text::Num(n) => n.to_string(),
        }
    }
}

/// `deserialize_with` for a `String` decimal field.
pub fn decimal<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Text::deserialize(d).map(Into::into)
}

/// `deserialize_with` for an `Option<String>` decimal field (`null` → `None`).
pub fn opt_decimal<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(Option::<Text>::deserialize(d)?.map(Into::into))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Deserialize)]
    struct Body {
        #[serde(deserialize_with = "decimal")]
        amount: String,
        #[serde(default, deserialize_with = "opt_decimal")]
        rate: Option<String>,
    }

    fn body(v: serde_json::Value) -> Body {
        serde_json::from_value(v).expect("parse")
    }

    #[test]
    fn numbers_keep_their_shortest_text() {
        assert_eq!(body(json!({ "amount": 1000 })).amount, "1000");
        assert_eq!(body(json!({ "amount": 0.1 })).amount, "0.1");
        assert_eq!(body(json!({ "amount": 1234.56 })).amount, "1234.56");
        assert_eq!(body(json!({ "amount": -12.5 })).amount, "-12.5");
        assert_eq!(body(json!({ "amount": "1210.00" })).amount, "1210.00");
    }

    #[test]
    fn optional_fields() {
        assert_eq!(body(json!({ "amount": 1 })).rate, None);
        assert_eq!(body(json!({ "amount": 1, "rate": null })).rate, None);
        assert_eq!(
            body(json!({ "amount": 1, "rate": 25.125 })).rate.as_deref(),
            Some("25.125")
        );
    }

    #[test]
    fn other_types_are_rejected() {
        assert!(serde_json::from_value::<Body>(json!({ "amount": true })).is_err());
        assert!(serde_json::from_value::<Body>(json!({ "amount": [1] })).is_err());
    }
}
