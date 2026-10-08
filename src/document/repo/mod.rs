//! Document persistence. Multi-row writes run in `db.transaction(…)`; the
//! document row is locked (`FOR UPDATE`) before a state check so concurrent
//! writes serialize on it.

pub mod advance_sources;
pub mod context;
pub mod credit;
pub mod ddpp;
pub mod issue;
pub mod lifecycle;
pub mod lines;
pub mod payments;
pub mod query;
pub mod settle;
pub mod view;
pub mod write;
