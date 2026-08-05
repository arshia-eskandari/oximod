//! Validation parsing and token generation.
//!
//! This module coordinates:
//!
//! - parsed validation arguments and numeric literal types;
//! - specialized validation-rule builders;
//! - field-type classification and numeric-bound helpers;
//! - generated runtime checks and compile-time diagnostics.
//!
//! Its exports are available only within `oximod_macros`.

mod args;
mod build_checks;
mod helpers;
mod macros;
mod model_tokens;

pub(crate) use args::{LitNum, ValidateArgs};

pub(crate) use model_tokens::generate_validate_model_tokens;
