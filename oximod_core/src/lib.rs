#![doc = include_str!("../README.md")]

/// Typed MongoDB aggregation pipeline construction and execution.
pub mod aggregation;

/// Error and validation types used by OxiMod operations.
pub mod error;

/// Runtime infrastructure for models, connections, hooks, and persistence.
///
/// This module is public so generated code can access it through the main
/// `oximod` crate. Applications should use the public re-exports from
/// `oximod` instead of depending on this module directly.
#[doc(hidden)]
pub mod feature;

/// Internal utilities shared by OxiMod's runtime and generated code.
#[doc(hidden)]
pub mod helpers;

/// Typed MongoDB query, sorting, update, deletion, and geospatial primitives.
pub mod query;
