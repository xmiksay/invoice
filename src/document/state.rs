//! Derived, never-stored document state: payment state and overdue.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::line::{PaymentState, Status};

/// Payments are always positive, so `paid >= 0`. A non-positive payable with
/// nothing paid counts as settled.
pub fn payment_state(paid: Decimal, payable: Decimal) -> PaymentState {
    if paid == payable || (paid.is_zero() && payable < Decimal::ZERO) {
        PaymentState::Paid
    } else if paid.is_zero() {
        PaymentState::Unpaid
    } else if paid < payable {
        PaymentState::Partial
    } else {
        PaymentState::Overpaid
    }
}

/// `paymentState` is only meaningful for issued documents.
pub fn document_payment_state(
    status: Status,
    paid: Decimal,
    payable: Decimal,
) -> Option<PaymentState> {
    (status == Status::Issued).then(|| payment_state(paid, payable))
}

/// Issued, still owing (unpaid or partial ⇔ `paid < payable`) and past due.
pub fn is_overdue(
    status: Status,
    paid: Decimal,
    payable: Decimal,
    due: NaiveDate,
    today: NaiveDate,
) -> bool {
    status == Status::Issued && paid < payable && due < today
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: i64) -> Decimal {
        Decimal::from(n)
    }

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).expect("date")
    }

    #[test]
    fn payment_states() {
        assert_eq!(payment_state(d(0), d(100)), PaymentState::Unpaid);
        assert_eq!(payment_state(d(40), d(100)), PaymentState::Partial);
        assert_eq!(payment_state(d(100), d(100)), PaymentState::Paid);
        assert_eq!(payment_state(d(101), d(100)), PaymentState::Overpaid);
        assert_eq!(payment_state(d(0), d(0)), PaymentState::Paid);
        assert_eq!(payment_state(d(0), d(-5)), PaymentState::Paid);
        assert_eq!(payment_state(d(1), d(-5)), PaymentState::Overpaid);
        assert_eq!(document_payment_state(Status::Draft, d(0), d(100)), None);
        assert_eq!(
            document_payment_state(Status::Cancelled, d(0), d(100)),
            None
        );
    }

    #[test]
    fn overdue_rules() {
        let today = date(10);
        assert!(is_overdue(Status::Issued, d(0), d(100), date(9), today));
        assert!(is_overdue(Status::Issued, d(50), d(100), date(9), today));
        assert!(!is_overdue(Status::Issued, d(0), d(100), date(10), today));
        assert!(!is_overdue(Status::Issued, d(100), d(100), date(9), today));
        assert!(!is_overdue(Status::Draft, d(0), d(100), date(9), today));
        assert!(!is_overdue(Status::Cancelled, d(0), d(100), date(9), today));
    }
}
