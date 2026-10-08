//! ARES (Czech business register) lookup by IČO → contact draft.

pub mod client;
pub mod handlers;

pub use client::{AresClient, AresSubject, DEFAULT_ARES_URL};
