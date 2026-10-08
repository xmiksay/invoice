//! Party defaults for new documents: the contact's own setting wins over the
//! company's.

use chrono::{Days, NaiveDate};

use crate::contact::entity::contact;
use crate::settings::entity::company;

/// `from` + (contact ?? company) default due days.
pub fn due_date(
    contact: Option<&contact::Model>,
    company: &company::Model,
    from: NaiveDate,
) -> NaiveDate {
    let days = contact
        .and_then(|c| c.default_due_days)
        .unwrap_or(company.default_due_days);
    from.checked_add_days(Days::new(u64::try_from(days).unwrap_or(0)))
        .unwrap_or(from)
}

/// (contact ?? company) default document locale.
pub fn locale(contact: Option<&contact::Model>, company: &company::Model) -> String {
    contact
        .and_then(|c| c.default_locale.clone())
        .unwrap_or_else(|| company.default_locale.clone())
}
