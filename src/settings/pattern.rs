//! Document number patterns, e.g. `{YYYY}{NNNN}` → `20260001`.
//!
//! Tokens: `{YYYY}` four-digit year, `{YY}` two-digit year, `{N…}` (1–9 `N`s)
//! the zero-padded sequence number. Exactly one year token and exactly one
//! sequence token are required — counters reset per year, so a pattern
//! without the year would repeat numbers. Everything else is a literal from
//! `[A-Za-z0-9/_-]`. At most [`MAX_LEN`] characters.

pub const MAX_LEN: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Literal(char),
    Year4,
    Year2,
    Seq(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid number pattern")]
pub struct InvalidPattern;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern(Vec<Token>);

impl Pattern {
    pub fn parse(s: &str) -> Result<Self, InvalidPattern> {
        if s.chars().count() > MAX_LEN {
            return Err(InvalidPattern);
        }
        let mut tokens = Vec::new();
        let mut rest = s;
        while let Some(c) = rest.chars().next() {
            if c == '{' {
                let end = rest.find('}').ok_or(InvalidPattern)?;
                tokens.push(placeholder(&rest[1..end])?);
                rest = &rest[end + 1..];
            } else if c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-') {
                tokens.push(Token::Literal(c));
                rest = &rest[c.len_utf8()..];
            } else {
                return Err(InvalidPattern);
            }
        }
        let seqs = tokens.iter().filter(|t| matches!(t, Token::Seq(_))).count();
        let years = tokens
            .iter()
            .filter(|t| matches!(t, Token::Year4 | Token::Year2))
            .count();
        if seqs != 1 || years != 1 {
            return Err(InvalidPattern);
        }
        Ok(Self(tokens))
    }

    /// Render the number. A sequence wider than its token keeps every digit.
    pub fn format(&self, year: i32, n: i64) -> String {
        self.0
            .iter()
            .map(|t| match t {
                Token::Literal(c) => c.to_string(),
                Token::Year4 => format!("{year:04}"),
                Token::Year2 => format!("{:02}", year.rem_euclid(100)),
                Token::Seq(width) => format!("{n:0width$}"),
            })
            .collect()
    }
}

fn placeholder(inner: &str) -> Result<Token, InvalidPattern> {
    match inner {
        "YYYY" => Ok(Token::Year4),
        "YY" => Ok(Token::Year2),
        n if (1..=9).contains(&n.len()) && n.bytes().all(|b| b == b'N') => Ok(Token::Seq(n.len())),
        _ => Err(InvalidPattern),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format_number(pattern: &str, year: i32, n: i64) -> Result<String, InvalidPattern> {
        Pattern::parse(pattern).map(|p| p.format(year, n))
    }

    #[test]
    fn formats_seeded_patterns() {
        assert_eq!(
            format_number("{YYYY}{NNNN}", 2026, 1),
            Ok("20260001".into())
        );
        assert_eq!(
            format_number("D{YYYY}{NNNN}", 2026, 42),
            Ok("D20260042".into())
        );
        assert_eq!(
            format_number("DP{YYYY}{NNNN}", 2026, 7),
            Ok("DP20260007".into())
        );
        assert_eq!(
            format_number("FV-{YY}/{NNN}", 2026, 5),
            Ok("FV-26/005".into())
        );
        assert_eq!(format_number("{YY}{N}", 2005, 3), Ok("053".into()));
        assert_eq!(
            format_number("{YY}{NNNNNNNNN}", 2026, 1),
            Ok("26000000001".into())
        );
    }

    #[test]
    fn sequence_overflow_keeps_all_digits() {
        assert_eq!(
            format_number("{YYYY}{NN}", 2026, 12345),
            Ok("202612345".into())
        );
    }

    #[test]
    fn year_is_required_and_literals_pass_through() {
        assert_eq!(
            format_number("A_b-1/{YY}/{NNN}", 2026, 9),
            Ok("A_b-1/26/009".into())
        );
        assert_eq!(format_number("A_b-1/{NNN}", 2026, 9), Err(InvalidPattern));
        assert_eq!(
            format_number("{YYYY}{YY}{NNN}", 2026, 9),
            Err(InvalidPattern)
        );
    }

    #[test]
    fn rejects_invalid_patterns() {
        for bad in [
            "",
            "{YYYY}",
            "{NNNN}{NN}",
            "{NNNN}",
            "{YY}{YY}{N}",
            "{YY}{NNNNNNNNNN}",
            "{N",
            "{}",
            "{YYY}{NNN}",
            "{YY}{nnn}",
            "{YY}{X}{NNN}",
            "{YY}FA {NNN}",
            "{YY}FA.{NNN}",
            "{YY}Č{NNN}",
            "{YY}}{NNN}",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA{YY}{NNN}",
        ] {
            assert_eq!(Pattern::parse(bad), Err(InvalidPattern), "{bad:?}");
        }
    }

    #[test]
    fn accepts_max_length() {
        let p = format!("{}{{YY}}{{N}}", "A".repeat(MAX_LEN - 7));
        assert_eq!(p.chars().count(), MAX_LEN);
        assert!(Pattern::parse(&p).is_ok());
        let p = format!("A{p}");
        assert_eq!(Pattern::parse(&p), Err(InvalidPattern));
    }
}
