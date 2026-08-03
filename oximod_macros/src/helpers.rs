//! Shared orchestration helpers for the `Model` derive macro.
//!
//! This module brings together the internal helpers responsible for:
//!
//! - collecting and validating model-level attributes;
//! - processing fields into setters, defaults, validations, and indexes;
//! - determining whether a model is collection-backed or embedded.
//!
//! Its exports are available only within `oximod_macros`.

mod model_attrs;
mod model_fields;
mod model_kind;

pub(crate) use model_attrs::{CollectionAttrs, ModelAttrs, collect_model_attrs};
pub(crate) use model_fields::{FieldTokenStreams, generate_field_tokens};
pub(crate) use model_kind::ModelKind;
