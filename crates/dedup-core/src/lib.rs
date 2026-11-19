//! Offline bibliographic record comparison tools.

pub mod identifier_review;
pub mod normalize;

/// Package version, not an accuracy or format-support claim.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
