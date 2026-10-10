//! Per-field validation failures (`AppError::Validation`).

use std::collections::BTreeMap;

use super::AppError;

/// Per-field validation failures: camelCase wire field name → reason code
/// (`required`, `invalid`, `too_long`, `duplicate`, `invalid_ico`, `invalid_pattern`,
/// `below_issued`, `exceeds_original`, `mixed_vat`, `unknown`, `inactive`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FieldErrors(pub(super) BTreeMap<String, &'static str>);

impl FieldErrors {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `reason` for `field`; the first reason reported for a field wins.
    pub fn add(&mut self, field: &str, reason: &'static str) {
        self.0.entry(field.to_string()).or_insert(reason);
    }

    /// Record the outcome of a field check (`Err(reason)`), ignoring `Ok`.
    pub fn check<T>(&mut self, field: &str, result: Result<T, &'static str>) -> Option<T> {
        match result {
            Ok(v) => Some(v),
            Err(reason) => {
                self.add(field, reason);
                None
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The reason recorded for `field`.
    pub fn get(&self, field: &str) -> Option<&'static str> {
        self.0.get(field).copied()
    }

    /// Add every error of `other` (existing reasons win).
    pub fn merge(&mut self, other: FieldErrors) {
        for (field, reason) in other.0 {
            self.0.entry(field).or_insert(reason);
        }
    }

    /// `Ok(())` when nothing was recorded, else [`AppError::Validation`].
    pub fn into_result(self) -> Result<(), AppError> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(AppError::Validation(self))
        }
    }
}
