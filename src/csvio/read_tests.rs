use rust_xlsxwriter::{Format, Workbook};

use super::*;

fn text(s: &str) -> Cell {
    Cell::Text(s.into())
}

fn d(s: &str) -> Decimal {
    s.parse().expect("decimal")
}

#[test]
fn utf8_with_bom_semicolons_and_crlf() {
    let t = read("\u{feff}Direction ; TOTAL;note\r\nissued;1210,00;\"a;\"\"b\"\"\"\r\n".as_bytes())
        .expect("table");
    assert_eq!(t.header, ["direction", "total", "note"]);
    assert_eq!(
        t.rows,
        [Record {
            line: 2,
            cells: vec![text("issued"), text("1210,00"), text("a;\"b\"")]
        }]
    );
}

#[test]
fn windows_1250_fallback() {
    // "Účetní" in Windows-1250.
    let mut bytes = b"direction;note\nissued;".to_vec();
    bytes.extend_from_slice(&[0xDA, 0xE8, 0x65, 0x74, 0x6E, 0xED]);
    let t = read(&bytes).expect("table");
    assert_eq!(t.rows[0].cells[1], text("Účetní"));
    // A UTF-8 BOM in front of Windows-1250 bytes is dropped, not decoded.
    let with_bom = [b"\xEF\xBB\xBF".as_slice(), &bytes].concat();
    let t = read(&with_bom).expect("table");
    assert_eq!(t.header, ["direction", "note"]);
    assert_eq!(t.rows[0].cells[1], text("Účetní"));
}

#[test]
fn separator_detection() {
    assert_eq!(separator("direction,total,base_21\n"), b',');
    assert_eq!(separator("direction\ttotal\n"), b'\t');
    assert_eq!(separator("direction;total\n"), b';');
    // Nothing known: `;`.
    assert_eq!(separator("a,b,c\n"), b';');
    let t = read(b"direction,total\nissued,\"1,5\"\n").expect("table");
    assert_eq!(t.rows[0].cells, [text("issued"), text("1,5")]);
}

#[test]
fn line_numbers_skip_empty_rows_and_count_quoted_lines() {
    let csv = "direction;note\n\n;\nissued;\"two\nlines\"\nreceived;x\n";
    let t = read(csv.as_bytes()).expect("table");
    let lines: Vec<u32> = t.rows.iter().map(|r| r.line).collect();
    assert_eq!(lines, [4, 6]);
}

#[test]
fn empty_and_too_many() {
    assert_eq!(read(b""), Err(FileError::Empty));
    let t = read(b"direction;total\n;\n").expect("table");
    assert!(t.rows.is_empty());
    let mut csv = String::from("direction\n");
    for _ in 0..MAX_ROWS {
        csv.push_str("issued\n");
    }
    assert_eq!(read(csv.as_bytes()).map(|t| t.rows.len()), Ok(MAX_ROWS));
    csv.push_str("issued\n");
    assert_eq!(read(csv.as_bytes()), Err(FileError::TooMany));
}

#[test]
fn not_an_xlsx() {
    assert_eq!(read(b"PK\x03\x04garbage"), Err(FileError::Invalid));
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("a.txt", zip::write::SimpleFileOptions::default())
        .expect("entry");
    let bytes = zip.finish().expect("zip").into_inner();
    assert_eq!(read(&bytes), Err(FileError::Invalid));
}

fn workbook() -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let date = Format::new().set_num_format("dd.mm.yyyy");
    let rows: [(&str, &str); 2] = [("Direction", "issued"), ("note", " x ")];
    for (c, (h, v)) in (0u16..).zip(rows) {
        ws.write_string(1, c, h).expect("header");
        ws.write_string(2, c, v).expect("cell");
    }
    ws.write_string(1, 2, "total").expect("h");
    ws.write_number(2, 2, 1234.5).expect("n");
    ws.write_string(1, 3, "base_21").expect("h");
    ws.write_number(2, 3, 0.1 + 0.2).expect("n");
    ws.write_string(1, 4, "issue_date").expect("h");
    let dt = rust_xlsxwriter::ExcelDateTime::from_ymd(2026, 1, 15).expect("date");
    ws.write_datetime_with_format(2, 4, &dt, &date)
        .expect("date");
    ws.write_string(1, 5, "counterparty_ico").expect("h");
    ws.write_number(2, 5, 6947.0).expect("n");
    ws.write_boolean(2, 6, true).expect("b");
    ws.write_string(4, 0, "received").expect("row 5");
    wb.add_worksheet()
        .write_string(0, 0, "ignored")
        .expect("2nd");
    wb.save_to_buffer().expect("xlsx")
}

#[test]
fn xlsx_first_sheet_numbers_and_dates() {
    let t = read(&workbook()).expect("table");
    // The used range starts at sheet row 2: that is the header.
    assert_eq!(
        t.header,
        [
            "direction",
            "note",
            "total",
            "base_21",
            "issue_date",
            "counterparty_ico",
        ]
    );
    assert_eq!(t.rows.len(), 2);
    let r = &t.rows[0];
    assert_eq!(r.line, 3);
    assert_eq!(r.cells[0], text("issued"));
    assert_eq!(r.cells[1], text("x"));
    assert_eq!(r.cells[2], Cell::Number(d("1234.5")));
    assert_eq!(r.cells[3], Cell::Number(d("0.3")));
    assert_eq!(
        r.cells[4],
        Cell::Date(NaiveDate::from_ymd_opt(2026, 1, 15).expect("date"))
    );
    assert_eq!(r.cells[5], Cell::Number(d("6947")));
    assert_eq!(r.cells[6], text("true"));
    assert_eq!(t.rows[1].line, 5);
}

fn sheet(cells: &[(u32, u16, &str)]) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for (r, c, v) in cells {
        ws.write_string(*r, *c, *v).expect("cell");
    }
    wb.save_to_buffer().expect("xlsx")
}

#[test]
fn xlsx_huge_used_range_is_never_allocated() {
    // A1 + XFD1048576: a few KB declaring 17 billion cells.
    let bytes = sheet(&[(0, 0, "direction"), (1_048_575, 16_383, "x")]);
    assert_eq!(read(&bytes), Err(FileError::Invalid));
    // Far apart rows within the column cap are fine (only used cells count).
    let t = read(&sheet(&[(0, 0, "direction"), (1_048_575, 0, "issued")])).expect("table");
    assert_eq!(
        t.rows,
        [Record {
            line: 1_048_576,
            cells: vec![text("issued")]
        }]
    );
    let wide = (MAX_COLUMNS - 1) as u16;
    assert!(read(&sheet(&[(0, wide, "direction"), (1, 0, "x")])).is_ok());
    assert_eq!(
        read(&sheet(&[(0, wide + 1, "direction")])),
        Err(FileError::Invalid)
    );
}

#[test]
fn xlsx_row_cap() {
    let mut cells = vec![(0u32, 0u16, "direction")];
    cells.extend((1..=MAX_ROWS as u32).map(|r| (r, 0u16, "issued")));
    assert_eq!(read(&sheet(&cells)).map(|t| t.rows.len()), Ok(MAX_ROWS));
    cells.push((MAX_ROWS as u32 + 1, 0, "issued"));
    assert_eq!(read(&sheet(&cells)), Err(FileError::TooMany));
    assert_eq!(read(&sheet(&[])), Err(FileError::Empty));
}
