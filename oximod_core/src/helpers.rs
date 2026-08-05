//! Internal runtime utilities shared by OxiMod.
//!
//! This module is public so generated code can access its helpers through the
//! main `oximod` crate. Applications should not normally depend on these
//! utilities directly.

/// Retry-aware asynchronous one-time initialization.
pub mod once_async;
