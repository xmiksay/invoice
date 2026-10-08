//! Calendar "today" for business rules (overdue, default dates, ČNB caching).
//!
//! Czech invoicing dates follow Prague time regardless of the server's (often
//! UTC) local zone.

use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::Europe::Prague;

pub fn today() -> NaiveDate {
    Utc::now().with_timezone(&Prague).date_naive()
}

pub fn current_year() -> i32 {
    today().year()
}
