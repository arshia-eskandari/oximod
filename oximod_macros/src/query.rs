//! Typed-query code generation for derived models.
//!
//! This module is the entry point used by the `Model` derive macro. The
//! implementation is divided by responsibility:
//!
//! - `field_schema` generates typed model-field structures;
//! - `queryable` generates collection-only `Queryable` implementations;
//! - `serde_name` resolves serialized MongoDB field names.

mod field_schema;
mod queryable;
mod serde_name;

use proc_macro2::TokenStream;
use syn::DeriveInput;

/// Generates a model's hidden typed field structure and `FieldSchema`
/// implementation.
///
/// Both collection and embedded models receive a field schema.
pub fn generate_field_schema_tokens(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    field_schema::generate_field_schema_tokens(input)
}

/// Generates the collection-only `Queryable` implementation for a model.
///
/// Embedded models receive a field schema but do not receive this
/// implementation.
pub fn generate_query_tokens(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    queryable::generate_query_tokens(input)
}
