//! Specialized validation-rule builders.
//!
//! Each submodule appends one category of generated validation tokens to
//! `BuiltChecks`:
//!
//! - `custom_checks` invokes user-defined validators;
//! - `length_types` handles length and non-empty rules;
//! - `numbers` handles numeric bounds, signs, and divisibility;
//! - `options` handles rules that inspect `Option<T>` directly;
//! - `strings` handles string content and format rules.
//!
//! These modules are visible only within the parent `validate` module.

pub(super) mod custom_checks;
pub(super) mod length_types;
pub(super) mod numbers;
pub(super) mod options;
pub(super) mod strings;
