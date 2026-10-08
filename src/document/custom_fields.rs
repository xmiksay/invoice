//! Custom field values of a document (`documents.custom_fields`) checked
//! against the active definitions that apply to its direction.

use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::{Map, Value};

use crate::error::FieldErrors;
use crate::validation::Check;

pub type Values = Map<String, Value>;

pub const MAX_TEXT: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Text,
    Number,
    Date,
    Bool,
    Select,
}

impl FieldType {
    pub const ALL: [FieldType; 5] = [
        FieldType::Text,
        FieldType::Number,
        FieldType::Date,
        FieldType::Bool,
        FieldType::Select,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::Number => "number",
            FieldType::Date => "date",
            FieldType::Bool => "bool",
            FieldType::Select => "select",
        }
    }

    pub fn parse(s: &str) -> Option<FieldType> {
        FieldType::ALL.into_iter().find(|t| t.as_str() == s)
    }
}

/// An active definition applying to the document's direction.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub key: String,
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub required: bool,
}

/// The normalized value, `None` when empty.
fn value(def: &FieldDef, v: &Value) -> Check<Option<Value>> {
    let text = |v: &Value| -> Check<Option<String>> {
        match v {
            Value::String(s) => Ok(Some(s.trim().to_string()).filter(|s| !s.is_empty())),
            _ => Err("invalid"),
        }
    };
    if v.is_null() {
        return Ok(None);
    }
    Ok(match def.field_type {
        FieldType::Text => match text(v)? {
            Some(s) if s.chars().count() > MAX_TEXT => return Err("too_long"),
            other => other.map(Value::String),
        },
        FieldType::Number => match text(v)? {
            Some(s) => {
                let d = Decimal::from_str(&s).map_err(|_| "invalid")?;
                Some(Value::String(d.normalize().to_string()))
            }
            None => None,
        },
        FieldType::Date => match text(v)? {
            Some(s) => {
                // Canonical only: chrono also accepts `2026-1-5`.
                let d = NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|_| "invalid")?;
                if d.format("%Y-%m-%d").to_string() != s {
                    return Err("invalid");
                }
                Some(Value::String(s))
            }
            None => None,
        },
        FieldType::Bool => match v {
            Value::Bool(b) => Some(Value::Bool(*b)),
            _ => return Err("invalid"),
        },
        FieldType::Select => match text(v)? {
            Some(s) if def.options.contains(&s) => Some(Value::String(s)),
            Some(_) => return Err("invalid"),
            None => None,
        },
    })
}

/// Validate `input` and return what to store. A value sent back exactly as
/// `stored` is kept without validation (its definition may have been
/// deactivated, deleted or changed since). Other values: `customFields.<key>`
/// `unknown` (no active definition), `invalid` / `too_long` (wrong value);
/// `required` when a required field ends up empty. A `null` for a stored key
/// without a definition clears it.
pub fn validate(input: &Values, defs: &[FieldDef], stored: &Values, e: &mut FieldErrors) -> Values {
    let mut out = Values::new();
    for (key, v) in input {
        let field = format!("customFields.{key}");
        if !v.is_null() && stored.get(key) == Some(v) {
            out.insert(key.clone(), v.clone());
            continue;
        }
        match defs.iter().find(|d| &d.key == key) {
            Some(def) => match value(def, v) {
                Ok(Some(n)) => {
                    out.insert(key.clone(), n);
                }
                Ok(None) => {}
                Err(r) => e.add(&field, r),
            },
            None if v.is_null() && stored.contains_key(key) => {}
            None => e.add(&field, "unknown"),
        }
    }
    for d in defs.iter().filter(|d| d.required) {
        if !out.contains_key(&d.key) && !input.get(&d.key).is_some_and(|v| value(d, v).is_err()) {
            e.add(&format!("customFields.{}", d.key), "required");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn def(key: &str, t: FieldType, required: bool) -> FieldDef {
        FieldDef {
            key: key.into(),
            field_type: t,
            options: vec!["a".into(), "b".into()],
            required,
        }
    }

    fn map(v: Value) -> Values {
        v.as_object().cloned().expect("object")
    }

    fn run(input: Value, defs: &[FieldDef], stored: Value) -> (Values, FieldErrors) {
        let mut e = FieldErrors::new();
        let out = validate(&map(input), defs, &map(stored), &mut e);
        (out, e)
    }

    fn errs(pairs: &[(&str, &'static str)]) -> FieldErrors {
        let mut e = FieldErrors::new();
        for (f, r) in pairs {
            e.add(f, r);
        }
        e
    }

    #[test]
    fn normalizes_each_type() {
        let defs = [
            def("t", FieldType::Text, false),
            def("n", FieldType::Number, false),
            def("d", FieldType::Date, false),
            def("b", FieldType::Bool, false),
            def("s", FieldType::Select, false),
        ];
        let (out, e) = run(
            json!({ "t": " x ", "n": "1.50", "d": "2026-10-01", "b": false, "s": "b" }),
            &defs,
            json!({}),
        );
        assert!(e.is_empty(), "{e:?}");
        assert_eq!(
            Value::Object(out),
            json!({ "t": "x", "n": "1.5", "d": "2026-10-01", "b": false, "s": "b" })
        );
        let (out, e) = run(json!({ "t": "", "n": null, "s": " " }), &defs, json!({}));
        assert!(e.is_empty() && out.is_empty());
    }

    #[test]
    fn rejects_wrong_values() {
        let defs = [
            def("t", FieldType::Text, false),
            def("n", FieldType::Number, false),
            def("d", FieldType::Date, false),
            def("b", FieldType::Bool, false),
            def("s", FieldType::Select, false),
        ];
        let long = "x".repeat(MAX_TEXT + 1);
        let (_, e) = run(
            json!({ "t": long, "n": "1,5", "d": "1.10.2026", "b": "true", "s": "c", "zz": 1 }),
            &defs,
            json!({}),
        );
        assert_eq!(
            e,
            errs(&[
                ("customFields.t", "too_long"),
                ("customFields.n", "invalid"),
                ("customFields.d", "invalid"),
                ("customFields.b", "invalid"),
                ("customFields.s", "invalid"),
                ("customFields.zz", "unknown"),
            ])
        );
        let (_, e) = run(json!({ "t": 5 }), &defs, json!({}));
        assert_eq!(e, errs(&[("customFields.t", "invalid")]));
        for bad in ["2026-1-5", "2026-01-5", " 2026-13-01", "+2026-01-05"] {
            let (_, e) = run(json!({ "d": bad }), &defs, json!({}));
            assert_eq!(e, errs(&[("customFields.d", "invalid")]), "{bad}");
        }
    }

    #[test]
    fn required_fields() {
        let defs = [def("r", FieldType::Text, true)];
        let (_, e) = run(json!({}), &defs, json!({}));
        assert_eq!(e, errs(&[("customFields.r", "required")]));
        let (_, e) = run(json!({ "r": "  " }), &defs, json!({}));
        assert_eq!(e, errs(&[("customFields.r", "required")]));
        let (out, e) = run(json!({ "r": "ok" }), &defs, json!({}));
        assert!(e.is_empty());
        assert_eq!(out["r"], "ok");
    }

    #[test]
    fn stored_values_of_inactive_fields_are_kept_only_unchanged() {
        let stored = json!({ "old": "v" });
        let (out, e) = run(json!({ "old": "v" }), &[], stored.clone());
        assert!(e.is_empty());
        assert_eq!(out["old"], "v");
        let (_, e) = run(json!({ "old": "w" }), &[], stored.clone());
        assert_eq!(e, errs(&[("customFields.old", "unknown")]));
        let (out, e) = run(json!({ "old": null }), &[], stored.clone());
        assert!(e.is_empty() && out.is_empty());
        let (out, e) = run(json!({}), &[], stored);
        assert!(e.is_empty() && out.is_empty());
    }

    #[test]
    fn unchanged_stored_values_skip_the_current_definition() {
        // Recreated as a number field; options changed on a select.
        let defs = [
            def("old", FieldType::Number, true),
            def("s", FieldType::Select, true),
        ];
        let stored = json!({ "old": "abc", "s": "gone" });
        let (out, e) = run(stored.clone(), &defs, stored.clone());
        assert!(e.is_empty(), "{e:?}");
        assert_eq!(Value::Object(out), stored);
        let (_, e) = run(json!({ "old": "abd", "s": "gone2" }), &defs, stored.clone());
        assert_eq!(
            e,
            errs(&[
                ("customFields.old", "invalid"),
                ("customFields.s", "invalid")
            ])
        );
        let (_, e) = run(json!({ "old": null, "s": "gone" }), &defs, stored);
        assert_eq!(e, errs(&[("customFields.old", "required")]));
    }
}
