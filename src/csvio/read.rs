//! An uploaded file → header + data rows: CSV (UTF-8, else Windows-1250;
//! separator detected from the header) or XLSX (first worksheet). Pure.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use calamine::{Data, Reader, Xlsx};
use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::format::{excel_number, is_fixed, rate_column};

/// More data rows → `too_many`.
pub const MAX_ROWS: usize = 500;
/// XLSX cells right of this column → `invalid` (the format has ~60 columns).
pub const MAX_COLUMNS: usize = 200;
/// Bytes actually decompressed from an XLSX (zip bombs).
const MAX_UNPACKED: u64 = 200 * 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;

/// A file-level error → 422 `{"fields":{"file":…}}` (`detail` for the
/// column ones).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileError {
    Invalid,
    Empty,
    TooMany,
    TooLarge,
    MissingColumn(String),
    InvalidColumn(String),
}

impl FileError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::Empty => "empty",
            Self::TooMany => "too_many",
            Self::TooLarge => "too_large",
            Self::MissingColumn(_) => "missing_column",
            Self::InvalidColumn(_) => "invalid_column",
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::MissingColumn(c) | Self::InvalidColumn(c) => Some(c),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Text(String),
    /// XLSX numeric cell.
    Number(Decimal),
    /// XLSX date cell.
    Date(NaiveDate),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// 1-based line (CSV) or sheet row (XLSX); the header is 1.
    pub line: u32,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Trimmed, lowercase.
    pub header: Vec<String>,
    /// Non-empty data rows, in file order.
    pub rows: Vec<Record>,
}

fn header_name(s: &str) -> String {
    s.trim()
        .trim_start_matches('\u{feff}')
        .trim()
        .to_lowercase()
}

fn is_known(name: &str) -> bool {
    is_fixed(name) || matches!(rate_column(name), Some(Ok(_)))
}

fn push(rows: &mut Vec<Record>, line: u32, cells: Vec<Cell>) -> Result<(), FileError> {
    if cells.iter().all(|c| *c == Cell::Empty) {
        return Ok(());
    }
    if rows.len() >= MAX_ROWS {
        return Err(FileError::TooMany);
    }
    rows.push(Record { line, cells });
    Ok(())
}

/// ZIP magic → XLSX, anything else is CSV text.
pub fn read(bytes: &[u8]) -> Result<Table, FileError> {
    if bytes.starts_with(b"PK\x03\x04") {
        xlsx(bytes)
    } else {
        // The `csv` crate miscounts lines ending in CRLF.
        csv_text(&decode(bytes).replace("\r\n", "\n").replace('\r', "\n"))
    }
}

/// UTF-8, else Windows-1250; a leading UTF-8 BOM is dropped either way.
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.trim_start_matches('\u{feff}').to_string(),
        // A UTF-8 BOM in front of Windows-1250 text is no character of it.
        Err(_) => encoding_rs::WINDOWS_1250
            .decode_without_bom_handling(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes))
            .0
            .into_owned(),
    }
}

fn reader(text: &str, sep: u8) -> csv::Reader<&[u8]> {
    csv::ReaderBuilder::new()
        .delimiter(sep)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes())
}

/// `;`, `,` or TAB: the one whose header line yields the most known
/// columns (ties in that order).
pub fn separator(text: &str) -> u8 {
    let known = |sep: u8| {
        reader(text, sep)
            .records()
            .next()
            .and_then(Result::ok)
            .map_or(0, |r| {
                r.iter().filter(|h| is_known(&header_name(h))).count()
            })
    };
    let mut best = (b';', known(b';'));
    for sep in *b",\t" {
        let n = known(sep);
        if n > best.1 {
            best = (sep, n);
        }
    }
    best.0
}

fn csv_text(text: &str) -> Result<Table, FileError> {
    let mut records = reader(text, separator(text)).into_records();
    let header = match records.next() {
        None => return Err(FileError::Empty),
        Some(r) => r.map_err(|_| FileError::Invalid)?,
    };
    let header = header.iter().map(header_name).collect();
    let mut rows = Vec::new();
    for r in records {
        let r = r.map_err(|_| FileError::Invalid)?;
        let line = r
            .position()
            .and_then(|p| u32::try_from(p.line()).ok())
            .unwrap_or(0);
        let cells = r
            .iter()
            .map(|c| match c.trim() {
                "" => Cell::Empty,
                t => Cell::Text(t.to_string()),
            })
            .collect();
        push(&mut rows, line, cells)?;
    }
    Ok(Table { header, rows })
}

/// Decompresses every entry once, counting the bytes actually produced.
fn check_unpacked(bytes: &[u8]) -> Result<(), FileError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| FileError::Invalid)?;
    if zip.len() > MAX_ENTRIES {
        return Err(FileError::Invalid);
    }
    let mut left = MAX_UNPACKED;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|_| FileError::Invalid)?;
        let n = std::io::copy(&mut entry.take(left + 1), &mut std::io::sink())
            .map_err(|_| FileError::Invalid)?;
        if n > left {
            return Err(FileError::TooLarge);
        }
        left -= n;
    }
    Ok(())
}

fn xlsx_cell(d: &Data) -> Cell {
    match d {
        Data::Empty => Cell::Empty,
        Data::String(s) if s.trim().is_empty() => Cell::Empty,
        Data::String(s) => Cell::Text(s.trim().to_string()),
        Data::Int(i) => Cell::Number(Decimal::from(*i)),
        Data::Float(f) => excel_number(*f).map_or_else(|| Cell::Text(f.to_string()), Cell::Number),
        Data::Bool(b) => Cell::Text(b.to_string()),
        Data::DateTime(dt) if dt.is_datetime() => {
            let (y, m, d, ..) = dt.to_ymd_hms_milli();
            NaiveDate::from_ymd_opt(i32::from(y), u32::from(m), u32::from(d))
                .map_or_else(|| Cell::Text(dt.as_f64().to_string()), Cell::Date)
        }
        Data::DateTimeIso(s) => s
            .get(..10)
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .map_or_else(|| Cell::Text(s.clone()), Cell::Date),
        other => Cell::Text(other.to_string()),
    }
}

/// Cells are streamed: the used range a sheet declares (A1:XFD1048576 in a
/// few bytes) is never allocated. Only non-empty cells are kept, at most
/// [`MAX_COLUMNS`] columns (else `invalid`) and the header plus
/// [`MAX_ROWS`] rows (else `too_many`).
fn xlsx(bytes: &[u8]) -> Result<Table, FileError> {
    check_unpacked(bytes)?;
    let mut book: Xlsx<_> = Xlsx::new(Cursor::new(bytes)).map_err(|_| FileError::Invalid)?;
    let sheet = book
        .sheet_names()
        .into_iter()
        .next()
        .ok_or(FileError::Invalid)?;
    let mut reader = book
        .worksheet_cells_reader(&sheet)
        .map_err(|_| FileError::Invalid)?;
    // Sheet row (0-based) → cells by column.
    let mut sheet_rows: BTreeMap<u32, Vec<Cell>> = BTreeMap::new();
    while let Some(c) = reader.next_cell().map_err(|_| FileError::Invalid)? {
        let cell = xlsx_cell(&Data::from(c.get_value().clone()));
        if cell == Cell::Empty {
            continue;
        }
        let (r, col) = c.get_position();
        let col = usize::try_from(col).map_err(|_| FileError::Invalid)?;
        if col >= MAX_COLUMNS {
            return Err(FileError::Invalid);
        }
        if !sheet_rows.contains_key(&r) && sheet_rows.len() > MAX_ROWS {
            return Err(FileError::TooMany);
        }
        let cells = sheet_rows.entry(r).or_default();
        if cells.len() <= col {
            cells.resize(col + 1, Cell::Empty);
        }
        cells[col] = cell;
    }
    let mut lines = sheet_rows.into_iter();
    let Some((_, header)) = lines.next() else {
        return Err(FileError::Empty);
    };
    let header = header
        .into_iter()
        .map(|c| match c {
            Cell::Text(t) => header_name(&t),
            _ => String::new(),
        })
        .collect();
    let mut rows = Vec::new();
    for (r, cells) in lines {
        // Sheet rows are 1-based.
        push(&mut rows, r.saturating_add(1), cells)?;
    }
    Ok(Table { header, rows })
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
