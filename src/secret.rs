use std::fmt;

use serde::Deserialize;

/// Wrapper that keeps sensitive values out of logs: `Debug` and `Display`
/// both print `[REDACTED]`. Use [`Secret::expose`] to read the inner value.
#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[cfg(test)]
mod tests {
    use super::Secret;

    #[test]
    fn debug_and_display_are_redacted() {
        let s = Secret::new("hunter2".to_string());
        assert_eq!(format!("{s:?}"), "[REDACTED]");
        assert_eq!(format!("{s}"), "[REDACTED]");
        assert_eq!(s.expose(), "hunter2");
    }

    #[test]
    fn redacted_inside_a_derived_debug() {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct Holder {
            token: Secret<String>,
        }
        let h = Holder {
            token: Secret::new("hunter2".into()),
        };
        assert!(!format!("{h:?}").contains("hunter2"));
    }
}
