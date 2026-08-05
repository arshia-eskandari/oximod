//! MongoDB index support for derived collection models.
//!
//! This module owns:
//!
//! - [`IndexArgs`], the internal representation populated from a field's
//!   `#[index(...)]` attribute;
//! - [`generate_index_model_tokens`], which turns those arguments into tokens
//!   that construct a MongoDB `IndexModel`.
//!
//! Attribute parsing remains in `crate::parsers::index`, while field-level
//! orchestration remains in `crate::helpers::model_fields`.
//!
//! Its exports are available only within `oximod_macros`.

mod args;
mod model_tokens;

pub(crate) use args::IndexArgs;
pub(crate) use model_tokens::generate_index_model_tokens;
