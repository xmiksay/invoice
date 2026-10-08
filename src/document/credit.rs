//! Pure credit-note rules: copying the invoice lines and the per-rate cap.

use rust_decimal::Decimal;

use super::line::LineData;

/// The invoice lines without its advance lines; subtotal refs are remapped to
/// the new positions (advance lines are never subtotal members).
pub fn copy_lines(lines: &[LineData]) -> Vec<LineData> {
    let mut new_pos = Vec::with_capacity(lines.len());
    let mut next = 0;
    for l in lines {
        if matches!(l, LineData::Advance(_)) {
            new_pos.push(None);
        } else {
            next += 1;
            new_pos.push(Some(next));
        }
    }
    lines
        .iter()
        .filter(|l| !matches!(l, LineData::Advance(_)))
        .map(|l| match l {
            LineData::Subtotal {
                description,
                refs,
                collapse,
            } => LineData::Subtotal {
                description: description.clone(),
                refs: refs
                    .iter()
                    .map(|r| {
                        usize::try_from(*r)
                            .ok()
                            .and_then(|p| p.checked_sub(1))
                            .and_then(|i| new_pos.get(i).copied().flatten())
                            .unwrap_or(*r)
                    })
                    .collect(),
                collapse: *collapse,
            },
            other => other.clone(),
        })
        .collect()
}

/// `true` when the credited bases `(rate, base)` — every non-cancelled credit
/// note of the invoice summed per rate — exceed the invoice's base for some
/// rate, or use a rate the invoice does not have. Overflow counts as exceeding.
pub fn exceeds_original(
    original: &[(Decimal, Decimal)],
    credited: impl IntoIterator<Item = (Decimal, Decimal)>,
) -> bool {
    let mut sums: Vec<(Decimal, Decimal)> = Vec::new();
    for (rate, base) in credited {
        match sums.iter_mut().find(|(r, _)| *r == rate) {
            Some((_, s)) => match s.checked_add(base) {
                Some(v) => *s = v,
                None => return true,
            },
            None => sums.push((rate, base)),
        }
    }
    sums.iter().any(|(rate, sum)| {
        original
            .iter()
            .find(|(r, _)| r == rate)
            .is_none_or(|(_, cap)| sum > cap)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::line::{AdvanceData, ItemData};
    use uuid::Uuid;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    fn item() -> LineData {
        LineData::Item(ItemData {
            description: "x".into(),
            quantity: d("1"),
            unit: None,
            unit_price: d("10"),
            discount_pct: d("0"),
            vat_rate: d("21"),
        })
    }

    #[test]
    fn copy_drops_advances_and_remaps_refs() {
        let adv = LineData::Advance(AdvanceData {
            document_id: Uuid::nil(),
            description: "a".into(),
            recap: vec![],
        });
        let sub = |refs: Vec<i32>| LineData::Subtotal {
            description: "s".into(),
            refs,
            collapse: true,
        };
        let lines = vec![adv.clone(), item(), adv, item(), sub(vec![2, 4])];
        assert_eq!(copy_lines(&lines), vec![item(), item(), sub(vec![1, 2])]);
    }

    #[test]
    fn cap_per_rate() {
        let original = [(d("21"), d("1000")), (d("12"), d("200"))];
        assert!(!exceeds_original(
            &original,
            [(d("21"), d("600")), (d("21"), d("400"))]
        ));
        assert!(exceeds_original(
            &original,
            [(d("21"), d("600")), (d("21"), d("400.01"))]
        ));
        assert!(!exceeds_original(&original, [(d("12"), d("200"))]));
        assert!(exceeds_original(&original, [(d("0"), d("1"))]));
        assert!(!exceeds_original(&original, []));
    }
}
