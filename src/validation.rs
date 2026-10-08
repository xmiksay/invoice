//! Pure field validators/normalizers shared by the domain modules.
//!
//! Each returns the normalized value or a reason code from the API contract
//! (`required`, `invalid`, `too_long`, `invalid_ico`), to be collected with
//! [`crate::error::FieldErrors::check`].

pub type Check<T> = Result<T, &'static str>;

fn too_long(s: &str, max: usize) -> bool {
    s.chars().count() > max
}

/// Trimmed, non-empty, at most `max` characters.
pub fn required_text(s: &str, max: usize) -> Check<String> {
    let s = s.trim();
    if s.is_empty() {
        Err("required")
    } else if too_long(s, max) {
        Err("too_long")
    } else {
        Ok(s.to_string())
    }
}

/// Trimmed, possibly empty, at most `max` characters.
pub fn text(s: &str, max: usize) -> Check<String> {
    let s = s.trim();
    if too_long(s, max) {
        Err("too_long")
    } else {
        Ok(s.to_string())
    }
}

/// Trimmed; empty or whitespace-only becomes `None`.
pub fn opt_text(s: Option<&str>, max: usize) -> Check<Option<String>> {
    match s.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => text(s, max).map(Some),
    }
}

/// Czech IČO: 8 digits, the last one a mod-11 check digit over weights 8..2.
pub fn is_valid_ico(ico: &str) -> bool {
    let digits: Vec<u32> = ico.chars().filter_map(|c| c.to_digit(10)).collect();
    if ico.len() != 8 || digits.len() != 8 {
        return false;
    }
    let sum: u32 = digits[..7]
        .iter()
        .zip((2..=8).rev())
        .map(|(d, w)| d * w)
        .sum();
    let check = match sum % 11 {
        0 => 1,
        1 => 0,
        r => 11 - r,
    };
    digits[7] == check
}

/// Optional IČO with inner whitespace removed.
pub fn opt_ico(s: Option<&str>) -> Check<Option<String>> {
    let Some(raw) = s else { return Ok(None) };
    let ico: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if ico.is_empty() {
        Ok(None)
    } else if is_valid_ico(&ico) {
        Ok(Some(ico))
    } else {
        Err("invalid_ico")
    }
}

/// Optional VAT id (DIČ): uppercase, no spaces, two-letter country prefix
/// followed by 2–12 alphanumerics (covers CZ and other EU VAT ids), <=14.
pub fn opt_dic(s: Option<&str>) -> Check<Option<String>> {
    let Some(raw) = s else { return Ok(None) };
    let dic: String = raw
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    if dic.is_empty() {
        return Ok(None);
    }
    if too_long(&dic, 14) {
        return Err("too_long");
    }
    let (prefix, rest) = dic.split_at_checked(2).ok_or("invalid")?;
    let ok = prefix.chars().all(|c| c.is_ascii_uppercase())
        && rest.len() >= 2
        && rest.chars().all(|c| c.is_ascii_alphanumeric());
    if ok { Ok(Some(dic)) } else { Err("invalid") }
}

/// IBAN with spaces removed and letters uppercased.
pub fn normalize_iban(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase()
}

/// Structure (country, check digits, 15–34 alphanumerics) and ISO 7064 mod-97.
/// Expects a normalized IBAN.
pub fn is_valid_iban(iban: &str) -> bool {
    let b = iban.as_bytes();
    if !(15..=34).contains(&b.len())
        || !b[..2].iter().all(u8::is_ascii_uppercase)
        || !b[2..4].iter().all(u8::is_ascii_digit)
        || !b
            .iter()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
    {
        return false;
    }
    let rearranged = b[4..].iter().chain(&b[..4]);
    let remainder = rearranged.fold(0u32, |acc, &c| {
        if c.is_ascii_digit() {
            (acc * 10 + u32::from(c - b'0')) % 97
        } else {
            (acc * 100 + u32::from(c - b'A' + 10)) % 97
        }
    });
    remainder == 1
}

pub fn opt_iban(s: Option<&str>) -> Check<Option<String>> {
    let Some(raw) = s else { return Ok(None) };
    let iban = normalize_iban(raw);
    if iban.is_empty() {
        Ok(None)
    } else if is_valid_iban(&iban) {
        Ok(Some(iban))
    } else {
        Err("invalid")
    }
}

/// Czech domestic account `[prefix-]number/bank`: `^(\d{1,6}-)?\d{2,10}/\d{4}$`.
pub fn is_valid_cz_account(s: &str) -> bool {
    let all_digits = |p: &str, min: usize, max: usize| {
        (min..=max).contains(&p.len()) && p.bytes().all(|c| c.is_ascii_digit())
    };
    let Some((account, bank)) = s.split_once('/') else {
        return false;
    };
    let number = match account.split_once('-') {
        Some((prefix, number)) if all_digits(prefix, 1, 6) => number,
        Some(_) => return false,
        None => account,
    };
    all_digits(number, 2, 10) && all_digits(bank, 4, 4)
}

pub fn opt_cz_account(s: Option<&str>) -> Check<Option<String>> {
    let Some(raw) = s else { return Ok(None) };
    let acc: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if acc.is_empty() {
        Ok(None)
    } else if is_valid_cz_account(&acc) {
        Ok(Some(acc))
    } else {
        Err("invalid")
    }
}

/// BIC/SWIFT: 8 or 11 alphanumerics, uppercased.
pub fn opt_bic(s: Option<&str>) -> Check<Option<String>> {
    let Some(raw) = s else { return Ok(None) };
    let bic = normalize_iban(raw);
    if bic.is_empty() {
        Ok(None)
    } else if matches!(bic.len(), 8 | 11) && bic.bytes().all(|c| c.is_ascii_alphanumeric()) {
        Ok(Some(bic))
    } else {
        Err("invalid")
    }
}

fn upper_letters(s: &str, len: usize) -> Check<String> {
    let s = s.trim().to_ascii_uppercase();
    if s.is_empty() {
        Err("required")
    } else if s.len() == len && s.bytes().all(|c| c.is_ascii_uppercase()) {
        Ok(s)
    } else {
        Err("invalid")
    }
}

/// ISO 4217 code: three letters, uppercased.
pub fn currency(s: &str) -> Check<String> {
    upper_letters(s, 3)
}

pub fn opt_currency(s: Option<&str>) -> Check<Option<String>> {
    match s.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => currency(s).map(Some),
    }
}

/// ISO 3166 alpha-2 country code, uppercased; empty defaults to `CZ`.
pub fn country(s: &str) -> Check<String> {
    if s.trim().is_empty() {
        return Ok("CZ".to_string());
    }
    upper_letters(s, 2)
}

pub const LOCALES: [&str; 2] = ["cs", "en"];

pub fn locale(s: &str) -> Check<String> {
    let s = s.trim();
    if LOCALES.contains(&s) {
        Ok(s.to_string())
    } else {
        Err("invalid")
    }
}

pub fn opt_locale(s: Option<&str>) -> Check<Option<String>> {
    match s.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => locale(s).map(Some),
    }
}

/// Loose e-mail check (something@something), <=200.
pub fn opt_email(s: Option<&str>) -> Check<Option<String>> {
    let email = opt_text(s, 200)?;
    if let Some(e) = &email {
        let valid = !e.contains(char::is_whitespace)
            && e.split_once('@')
                .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty());
        if !valid {
            return Err("invalid");
        }
    }
    Ok(email)
}

/// Default payment term in days.
pub fn due_days(n: i32) -> Check<i32> {
    if (0..=365).contains(&n) {
        Ok(n)
    } else {
        Err("invalid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ico_checksum() {
        for ok in [
            "27074358", "25596641", "45274649", "00006947", "12345679", "11111119",
        ] {
            assert!(is_valid_ico(ok), "{ok}");
        }
        for bad in [
            "27074357",
            "1234567",
            "123456789",
            "2707435a",
            "",
            "２７０７４３５８",
        ] {
            assert!(!is_valid_ico(bad), "{bad}");
        }
        assert_eq!(opt_ico(Some(" 270 74358 ")), Ok(Some("27074358".into())));
        assert_eq!(opt_ico(Some("  ")), Ok(None));
        assert_eq!(opt_ico(None), Ok(None));
        assert_eq!(opt_ico(Some("12345678")), Err("invalid_ico"));
    }

    #[test]
    fn iban_mod97_and_normalization() {
        assert_eq!(
            normalize_iban(" cz65 0800 0000 1920 0014 5399 "),
            "CZ6508000000192000145399"
        );
        for ok in [
            "CZ6508000000192000145399",
            "CZ6907101781240000004159",
            "DE89370400440532013000",
            "GB82WEST12345698765432",
        ] {
            assert!(is_valid_iban(ok), "{ok}");
        }
        for bad in [
            "CZ6508000000192000145398",
            "CZ65",
            "6508000000192000145399CZ",
            "CZ65-0800000019200014539",
        ] {
            assert!(!is_valid_iban(bad), "{bad}");
        }
        assert_eq!(
            opt_iban(Some("cz65 0800 0000 1920 0014 5399")),
            Ok(Some("CZ6508000000192000145399".into()))
        );
        assert_eq!(opt_iban(Some("CZ00 1234")), Err("invalid"));
        assert_eq!(opt_iban(Some("")), Ok(None));
    }

    #[test]
    fn cz_account_format() {
        for ok in [
            "19-2000145399/0800",
            "2000145399/0800",
            "12/0100",
            "123456-12/0100",
        ] {
            assert!(is_valid_cz_account(ok), "{ok}");
        }
        for bad in [
            "1/0100",
            "12345678901/0100",
            "1234567-12/0100",
            "-12/0100",
            "12/100",
            "12/01000",
            "12-0100",
            "ab/0100",
            "1-2-34/0100",
        ] {
            assert!(!is_valid_cz_account(bad), "{bad}");
        }
        assert_eq!(
            opt_cz_account(Some(" 19-2000145399/0800 ")),
            Ok(Some("19-2000145399/0800".into()))
        );
    }

    #[test]
    fn codes() {
        assert_eq!(currency(" czk "), Ok("CZK".into()));
        assert_eq!(currency("EU"), Err("invalid"));
        assert_eq!(currency("E1R"), Err("invalid"));
        assert_eq!(currency(""), Err("required"));
        assert_eq!(opt_currency(Some("")), Ok(None));
        assert_eq!(country(""), Ok("CZ".into()));
        assert_eq!(country("sk"), Ok("SK".into()));
        assert_eq!(country("SVK"), Err("invalid"));
        assert_eq!(locale("en"), Ok("en".into()));
        assert_eq!(locale("de"), Err("invalid"));
        assert_eq!(opt_locale(None), Ok(None));
        assert_eq!(opt_bic(Some("gibaczpx")), Ok(Some("GIBACZPX".into())));
        assert_eq!(opt_bic(Some("GIBACZPXXXX")), Ok(Some("GIBACZPXXXX".into())));
        assert_eq!(opt_bic(Some("GIBACZ")), Err("invalid"));
    }

    #[test]
    fn dic_format() {
        assert_eq!(opt_dic(Some("cz 12345678")), Ok(Some("CZ12345678".into())));
        assert_eq!(
            opt_dic(Some("CZ1234567890")),
            Ok(Some("CZ1234567890".into()))
        );
        assert_eq!(
            opt_dic(Some("NL123456789B01")),
            Ok(Some("NL123456789B01".into()))
        );
        assert_eq!(opt_dic(Some("NL123456789B012")), Err("too_long"));
        assert_eq!(opt_dic(Some("12345678")), Err("invalid"));
        assert_eq!(opt_dic(Some("CZ1")), Err("invalid"));
        assert_eq!(opt_dic(Some("C")), Err("invalid"));
        assert_eq!(opt_dic(Some(" ")), Ok(None));
    }

    #[test]
    fn texts() {
        assert_eq!(required_text("  a ", 5), Ok("a".into()));
        assert_eq!(required_text("   ", 5), Err("required"));
        assert_eq!(required_text("ěščřžý", 5), Err("too_long"));
        assert_eq!(required_text("ěščřž", 5), Ok("ěščřž".into()));
        assert_eq!(text("", 5), Ok(String::new()));
        assert_eq!(opt_text(Some("  "), 5), Ok(None));
        assert_eq!(opt_text(Some("abcdef"), 5), Err("too_long"));
        assert_eq!(opt_email(Some("a@b.cz")), Ok(Some("a@b.cz".into())));
        assert_eq!(opt_email(Some("ab.cz")), Err("invalid"));
        assert_eq!(opt_email(Some("@b.cz")), Err("invalid"));
        assert_eq!(opt_email(Some("a b@c.cz")), Err("invalid"));
        assert_eq!(due_days(365), Ok(365));
        assert_eq!(due_days(-1), Err("invalid"));
        assert_eq!(due_days(366), Err("invalid"));
    }
}
