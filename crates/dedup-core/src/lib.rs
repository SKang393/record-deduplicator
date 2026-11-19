//! Bibliographic record deduplication engine foundation.
//!
//! Matching, parsing, and export are not implemented yet. The behavioral
//! contract and implementation order are documented in the repository.

pub mod normalize;

/// Version of this foundation build, not an accuracy or format-support claim.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
