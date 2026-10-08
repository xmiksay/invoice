//! Subtotal-line validation and evaluation (port of infra `repo/subtotals.rs`).
//!
//! A subtotal references other lines by 1-based position, possibly other
//! subtotals. Rules: every ref points at an existing item or subtotal line
//! other than itself, no duplicate refs, no reference cycles, and every
//! (transitively) referenced item shares one VAT rate. A subtotal's rate is
//! that shared rate; its base is the sum of the referenced bases.

use rust_decimal::Decimal;

use super::line::LineData;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Unvisited,
    InProgress,
    Done,
}

/// `(vat_rate, base)` per line (`None` for text lines).
pub type Resolved = Vec<Option<(Decimal, Decimal)>>;

/// Resolve every line given the item bases (`item_bases[i]` is used for item
/// lines only). On a violation returns the index of the offending subtotal.
pub fn resolve(lines: &[LineData], item_bases: &[Decimal]) -> Result<Resolved, usize> {
    let mut state = vec![State::Unvisited; lines.len()];
    let mut out: Resolved = vec![None; lines.len()];
    for idx in 0..lines.len() {
        resolve_one(idx, lines, item_bases, &mut state, &mut out)?;
    }
    Ok(out)
}

fn resolve_one(
    idx: usize,
    lines: &[LineData],
    item_bases: &[Decimal],
    state: &mut [State],
    out: &mut Resolved,
) -> Result<Option<(Decimal, Decimal)>, usize> {
    let refs = match &lines[idx] {
        LineData::Item(item) => {
            let base = item_bases.get(idx).copied().unwrap_or_default();
            out[idx] = Some((item.vat_rate, base));
            return Ok(out[idx]);
        }
        LineData::Text { .. } => return Ok(None),
        LineData::Subtotal { refs, .. } => refs,
    };
    match state[idx] {
        State::Done => return Ok(out[idx]),
        State::InProgress => return Err(idx),
        State::Unvisited => {}
    }
    state[idx] = State::InProgress;

    if refs.is_empty() {
        return Err(idx);
    }
    let mut rate: Option<Decimal> = None;
    let mut base = Decimal::ZERO;
    for (n, &pos) in refs.iter().enumerate() {
        let target = usize::try_from(pos)
            .ok()
            .and_then(|p| p.checked_sub(1))
            .filter(|&t| t < lines.len() && t != idx)
            .ok_or(idx)?;
        if refs[..n].contains(&pos) {
            return Err(idx);
        }
        let (member_rate, member_base) =
            resolve_one(target, lines, item_bases, state, out)?.ok_or(idx)?;
        match rate {
            Some(r) if r != member_rate => return Err(idx),
            _ => rate = Some(member_rate),
        }
        // Nested subtotals can grow without bound; overflow is a bad ref tree.
        base = base.checked_add(member_base).ok_or(idx)?;
    }
    let resolved = rate.map(|r| (r, base));
    out[idx] = resolved;
    state[idx] = State::Done;
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::line::ItemData;

    fn dec(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    fn item(rate: &str) -> LineData {
        LineData::Item(ItemData {
            description: "x".into(),
            quantity: Decimal::ONE,
            unit: None,
            unit_price: Decimal::ONE,
            discount_pct: Decimal::ZERO,
            vat_rate: dec(rate),
        })
    }

    fn sub(refs: &[i32]) -> LineData {
        LineData::Subtotal {
            description: String::new(),
            refs: refs.to_vec(),
            collapse: false,
        }
    }

    fn text() -> LineData {
        LineData::Text {
            description: "note".into(),
        }
    }

    fn run(lines: &[LineData]) -> Result<Resolved, usize> {
        let bases: Vec<Decimal> = (1..=lines.len()).map(|i| Decimal::from(i * 10)).collect();
        resolve(lines, &bases)
    }

    #[test]
    fn uniform_rate_sums_bases() {
        let r = run(&[item("21"), item("21"), sub(&[1, 2])]).expect("valid");
        assert_eq!(r[2], Some((dec("21"), dec("30"))));
        assert_eq!(r[0], Some((dec("21"), dec("10"))));
    }

    #[test]
    fn nested_subtotals_resolve() {
        let lines = [
            item("21"),
            item("21"),
            sub(&[1, 2]),
            item("21"),
            sub(&[3, 4]),
        ];
        let r = run(&lines).expect("valid");
        assert_eq!(r[4], Some((dec("21"), dec("70"))));
        // A subtotal may come before the lines it references.
        let r = run(&[sub(&[2, 3]), item("12"), item("12")]).expect("valid");
        assert_eq!(r[0], Some((dec("12"), dec("50"))));
    }

    #[test]
    fn mixed_rates_are_rejected_on_the_subtotal() {
        assert_eq!(run(&[item("21"), item("12"), sub(&[1, 2])]), Err(2));
        let nested = [item("21"), item("12"), sub(&[2]), sub(&[1, 3])];
        assert_eq!(run(&nested), Err(3));
    }

    #[test]
    fn cycles_and_self_refs_are_rejected() {
        assert!(run(&[sub(&[2]), sub(&[1])]).is_err());
        assert_eq!(run(&[item("21"), sub(&[2])]), Err(1));
        assert!(run(&[sub(&[2]), sub(&[3]), sub(&[1])]).is_err());
    }

    #[test]
    fn bad_refs_are_rejected() {
        assert_eq!(run(&[item("21"), sub(&[])]), Err(1));
        assert_eq!(run(&[item("21"), sub(&[99])]), Err(1));
        assert_eq!(run(&[item("21"), sub(&[0])]), Err(1));
        assert_eq!(run(&[item("21"), sub(&[-1])]), Err(1));
        assert_eq!(run(&[item("21"), sub(&[1, 1])]), Err(1));
        assert_eq!(run(&[text(), item("21"), sub(&[1, 2])]), Err(2));
    }

    #[test]
    fn text_lines_resolve_to_none() {
        assert_eq!(run(&[text(), item("0")]).expect("valid")[0], None);
    }
}
