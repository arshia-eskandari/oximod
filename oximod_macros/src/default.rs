//! Builder-setter generation for derived models.
//!
//! This module generates:
//!
//! - ordinary fluent field setters;
//! - the configurable MongoDB document-ID setter.
//!
//! Its exports are available only within `oximod_macros`.

mod field_setter;
mod id_setter;

pub(crate) use field_setter::push_field_setter;
pub(crate) use id_setter::push_id_setter;
