//! Runtime infrastructure used by OxiMod models.
//!
//! This module contains MongoDB client management, lifecycle hooks, and the
//! traits implemented by generated collection-backed and embedded models.
//!
//! Most applications should use the corresponding re-exports from the main
//! `oximod` crate rather than importing these modules directly.

/// MongoDB client initialization and access.
pub mod conn;

/// Lifecycle hooks for collection-backed models.
pub mod hooks;

/// Runtime traits and mode markers used by generated models.
pub mod model;

/// Path-aware nested validation runtime for `#[validate(nested)]`.
pub mod nested;
