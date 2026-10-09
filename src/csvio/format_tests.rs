use super::*;

fn d(s: &str) -> Decimal {
    s.parse().expect("decimal")
}

#[test]
fn rate_keys_and_columns() {
    assert_eq!(rate_key(d("21.00")), "21");
    assert_eq!(rate_key(d("12.50")), "12_5");
    assert_eq!(rate_key(d("0")), "0");
    assert_eq!(base_column(d("10.25")), "base_10_25");
    assert_eq!(vat_column(d("21")), "vat_21");
    assert_eq!(rate_column("base_21"), Some(Ok((RatePart::Base, d("21")))));
    assert_eq!(
        rate_column("vat_12_5"),
        Some(Ok((RatePart::Vat, d("12.5"))))
    );
    assert_eq!(
        rate_column("vat_12.5"),
        Some(Ok((RatePart::Vat, d("12.5"))))
    );
    assert_eq!(
        rate_column("base_21_0"),
        Some(Ok((RatePart::Base, d("21"))))
    );
    assert_eq!(rate_column("base_0"), Some(Ok((RatePart::Base, d("0")))));
    for bad in [
        "base_",
        "base_abc",
        "vat_21_",
        "vat__5",
        "base_1_2_3",
        "base_101",
        "base_1_234",
    ] {
        assert_eq!(rate_column(bad), Some(Err(())), "{bad}");
    }
    assert_eq!(rate_column("total"), None);
    assert_eq!(rate_column("basement"), None);
}

#[test]
fn numbers_with_both_decimal_marks() {
    let cases = [
        ("1234,50", "1234.50"),
        ("1234.50", "1234.50"),
        ("1 234,50", "1234.50"),
        ("1\u{a0}234,5", "1234.5"),
        ("1.234,50", "1234.50"),
        ("1,234.50", "1234.50"),
        ("1.234.567,89", "1234567.89"),
        ("-1210,00", "-1210.00"),
        ("\u{2212}5", "-5"),
        ("+7", "7"),
        ("0", "0"),
        ("24,335", "24.335"),
    ];
    for (input, want) in cases {
        assert_eq!(parse_decimal(input), Some(d(want)), "{input}");
    }
    for bad in [
        "",
        "-",
        "abc",
        "12a",
        "1.234.567",
        "1,2,3",
        ",5",
        "5,",
        "1e3",
        "12 Kč",
        "--1",
    ] {
        assert_eq!(parse_decimal(bad), None, "{bad}");
    }
}

#[test]
fn excel_numbers_are_exact() {
    assert_eq!(excel_number(1234.5), Some(d("1234.5")));
    assert_eq!(excel_number(0.1 + 0.2), Some(d("0.3")));
    assert_eq!(excel_number(1234.56), Some(d("1234.56")));
    assert_eq!(excel_number(1.1 * 3.0), Some(d("3.3")));
    assert_eq!(excel_number(-1210.0), Some(d("-1210")));
    assert_eq!(excel_number(24.335), Some(d("24.335")));
    assert_eq!(excel_number(12_345_679.0), Some(d("12345679")));
    assert_eq!(excel_number(f64::NAN), None);
    assert_eq!(excel_number(f64::INFINITY), None);
}

#[test]
fn dates() {
    let want = NaiveDate::from_ymd_opt(2026, 1, 5).expect("date");
    for s in [
        "5.1.2026",
        "05.01.2026",
        "5. 1. 2026",
        " 2026-01-05 ",
        "2026-1-5",
    ] {
        assert_eq!(parse_date(s), Some(want), "{s}");
    }
    for bad in [
        "31.2.2026",
        "5.1.26",
        "2026/01/05",
        "5.1",
        "x.1.2026",
        "",
        "26-01-05",
    ] {
        assert_eq!(parse_date(bad), None, "{bad}");
    }
    assert_eq!(format_date(want), "05.01.2026");
}

#[test]
fn booleans() {
    for t in ["1", "ano", "TRUE", "Yes"] {
        assert_eq!(parse_bool(t), Some(true), "{t}");
    }
    for f in ["0", "NE", "false", "no"] {
        assert_eq!(parse_bool(f), Some(false), "{f}");
    }
    assert_eq!(parse_bool("maybe"), None);
    assert_eq!(format_bool(true), "1");
}

#[test]
fn written_numbers() {
    assert_eq!(format_amount(d("1234.5")), "1234,50");
    assert_eq!(format_amount(d("-1210")), "-1210,00");
    assert_eq!(format_amount(d("0")), "0,00");
    assert_eq!(format_amount(-d("0")), "0,00");
    assert_eq!(format_rate(d("24.335000")), "24,335");
    assert_eq!(format_rate(d("25")), "25");
    // What is written reads back.
    assert_eq!(parse_decimal(&format_amount(d("-0.5"))), Some(d("-0.50")));
}

#[test]
fn fixed_columns_are_unique() {
    let mut all: Vec<&str> = BEFORE_RATES.iter().chain(&AFTER_RATES).copied().collect();
    let n = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), n);
    assert!(all.iter().all(|c| !matches!(rate_column(c), Some(Ok(_)))));
}

#[test]
fn formula_guard_round_trips() {
    for s in [
        "=1+1", "+420", "-5", "@x", "\tx", "\rx", "'=x", "''+x", "'", "'abc", "abc", "", "a=b",
    ] {
        assert_eq!(unguard_text(&guard_text(s)), s, "{s:?}");
    }
    assert_eq!(guard_text("=1+1"), "'=1+1");
    assert_eq!(guard_text("'abc"), "'abc");
    assert_eq!(guard_text("Firma"), "Firma");
    assert_eq!(unguard_text("'abc"), "'abc");
}
