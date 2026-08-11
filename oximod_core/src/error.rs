//! Error types used by OxiMod.
//!
//! This module contains the primary runtime error type, validation errors,
//! and errors produced by invalid typed-query configuration.

/// Internal failure-class classification for MongoDB driver errors.
///
/// Hidden macro-support infrastructure, not supported public API.
#[doc(hidden)]
pub mod classify;

/// Runtime and validation errors returned by OxiMod operations.
pub mod oximod_error;

/// Errors produced by invalid typed-query configuration.
pub mod query_error;
