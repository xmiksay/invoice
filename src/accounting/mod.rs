//! Accounting program exports ([api/accounting.md](../../docs/api/accounting.md)):
//! Settings → Accounting (the codes per direction × document type) and the
//! Pohoda XML writer, a format of the accountant export
//! ([`crate::csvio::export`]).

pub mod entity;
pub mod handlers;
pub mod pohoda;
pub mod pohoda_export;
pub mod pohoda_summary;
pub mod repo;
pub mod settings;
