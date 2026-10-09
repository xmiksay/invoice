//! Typed reads of one data row's cells, failing with the row error code and
//! the column. Pure.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::columns::Columns;
use super::format::{parse_date, parse_decimal, unguard_text};
use super::read::Cell;
use crate::import::model::Code;

pub const MISSING_FIELD: Code = "missing_field";
pub const INVALID_VALUE: Code = "invalid_value";
pub const INVALID_DATE: Code = "invalid_date";
pub const INVALID_AMOUNT: Code = "invalid_amount";
pub const NO_VAT_ROWS: Code = "no_vat_rows";
pub const TOTAL_MISMATCH: Code = "total_mismatch";
pub const NOT_ALLOWED: Code = "not_allowed";

/// Amounts must stay below 10^12.
const MAX: Decimal = Decimal::from_parts(0xD4A5_1000, 0xE8, 0, false, 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowError {
    pub code: Code,
    /// The CSV column the error is about.
    pub field: Option<String>,
}

impl RowError {
    pub fn new(code: Code, field: &str) -> Self {
        Self {
            code,
            field: Some(field.to_string()),
        }
    }
}

pub type Check<T> = Result<T, RowError>;

/// One data row.
#[derive(Clone, Copy)]
pub struct Row<'a> {
    pub cols: &'a Columns,
    pub cells: &'a [Cell],
}

/// The cell as text: trimmed, `None` when empty; an XLSX number as written
/// without trailing zeros, a date as `yyyy-mm-dd`.
pub fn text_of(cell: &Cell) -> Option<String> {
    match cell {
        Cell::Empty => None,
        Cell::Text(t) => Some(unguard_text(t.trim()).to_string()).filter(|t| !t.is_empty()),
        Cell::Number(n) => Some(n.normalize().to_string()),
        Cell::Date(d) => Some(d.to_string()),
    }
}

/// An amount: ≤ 2 dp (`dp`), |x| < 10^12 → else `invalid_amount`.
pub fn amount_of(cell: &Cell, field: &str, dp: u32) -> Check<Option<Decimal>> {
    let bad = || RowError::new(INVALID_AMOUNT, field);
    let x = match cell {
        Cell::Empty => return Ok(None),
        Cell::Number(n) => *n,
        Cell::Text(t) => parse_decimal(t).ok_or_else(bad)?,
        Cell::Date(_) => return Err(bad()),
    };
    if x.normalize().scale() > dp || x.abs() >= MAX {
        return Err(bad());
    }
    Ok(Some(x.normalize()))
}

impl Row<'_> {
    pub fn cell(&self, name: &str) -> &Cell {
        self.cols.cell(self.cells, name)
    }

    pub fn text(&self, name: &str) -> Option<String> {
        text_of(self.cell(name))
    }

    /// Optional text of at most `max` characters (`invalid_value`).
    pub fn opt_text(&self, name: &str, max: usize) -> Check<Option<String>> {
        match self.text(name) {
            Some(t) if t.chars().count() > max => Err(RowError::new(INVALID_VALUE, name)),
            t => Ok(t),
        }
    }

    pub fn req_text(&self, name: &str, max: usize) -> Check<String> {
        self.opt_text(name, max)?
            .ok_or_else(|| RowError::new(MISSING_FIELD, name))
    }

    /// One of `allowed` (case-insensitive); `None` when empty.
    pub fn choice<T>(&self, name: &str, parse: impl Fn(&str) -> Option<T>) -> Check<Option<T>> {
        match self.text(name) {
            None => Ok(None),
            Some(t) => parse(&t.to_lowercase())
                .map(Some)
                .ok_or_else(|| RowError::new(INVALID_VALUE, name)),
        }
    }

    pub fn date(&self, name: &str) -> Check<Option<NaiveDate>> {
        match self.cell(name) {
            Cell::Empty => Ok(None),
            Cell::Date(d) => Ok(Some(*d)),
            Cell::Text(t) => parse_date(t)
                .map(Some)
                .ok_or_else(|| RowError::new(INVALID_DATE, name)),
            Cell::Number(_) => Err(RowError::new(INVALID_DATE, name)),
        }
    }

    pub fn amount(&self, name: &str) -> Check<Option<Decimal>> {
        amount_of(self.cell(name), name, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_is_ten_to_the_twelfth() {
        assert_eq!(MAX, Decimal::from(1_000_000_000_000u64));
    }

    #[test]
    fn amounts() {
        let d = |s: &str| s.parse::<Decimal>().expect("d");
        let a = |c: Cell| amount_of(&c, "total", 2);
        assert_eq!(a(Cell::Text("1 234,50".into())), Ok(Some(d("1234.5"))));
        assert_eq!(a(Cell::Number(d("0.30"))), Ok(Some(d("0.3"))));
        assert_eq!(a(Cell::Empty), Ok(None));
        let bad = Err(RowError::new(INVALID_AMOUNT, "total"));
        assert_eq!(a(Cell::Text("1,234".into())), bad);
        assert_eq!(a(Cell::Text("1000000000000".into())), bad);
        assert_eq!(a(Cell::Text("x".into())), bad);
        assert_eq!(a(Cell::Date(NaiveDate::MIN)), bad);
        assert_eq!(
            a(Cell::Text("999999999999,99".into())),
            Ok(Some(d("999999999999.99")))
        );
        assert_eq!(
            amount_of(&Cell::Text("24,335".into()), "r", 6),
            Ok(Some(d("24.335")))
        );
    }

    #[test]
    fn texts() {
        assert_eq!(
            text_of(&Cell::Number("12345679.0".parse().expect("d"))),
            Some("12345679".into())
        );
        assert_eq!(text_of(&Cell::Text("  ".into())), None);
        let t = |s: &str| text_of(&Cell::Text(s.into()));
        assert_eq!(t("'=SUM(A1)"), Some("=SUM(A1)".into()));
        assert_eq!(t("''-x"), Some("'-x".into()));
        assert_eq!(
            t("'abc"),
            Some("'abc".into()),
            "only before a formula start"
        );
    }
}
