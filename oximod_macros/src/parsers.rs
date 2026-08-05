//! Attribute-parsing utilities for the `Model` derive macro.
//!
//! This module brings together parsers for:
//!
//! - single-value derive attributes;
//! - field defaults;
//! - MongoDB index configuration;
//! - numeric literals used by validation ranges;
//! - optional field types;
//! - field validation rules.
//!
//! The parsing macro remains in a private child module and is imported
//! explicitly by the parser that uses it.
//!
//! Its exports are available only within `oximod_macros`.

mod attr_value;
mod default;
mod index;
mod literals;
mod macros;
mod option;
mod validate;

pub(crate) use attr_value::parse_attr_value_ts;
pub(crate) use default::parse_default_expr;
pub(crate) use index::parse_index_args;
pub(crate) use literals::{parse_f64_for_range, parse_u128_for_range};
pub(crate) use option::{option_inner_type, unwrap_option_type};
pub(crate) use validate::parse_validate_args;
