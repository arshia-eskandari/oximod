//! Type-safe MongoDB queries and updates.
//!
//! This module contains the runtime building blocks for OxiMod's typed-query
//! API. Applications normally begin with [`Queryable::query`] and construct
//! filters, sorting, pagination, updates, deletions, text searches, and
//! geospatial queries through generated [`Field`] values.
//!
//! Public query types are re-exported from the main `oximod` crate, so
//! applications should normally import them directly from `oximod`.
//!
//! # Example
//!
//! ```ignore
//! let users = User::query()
//!     .filter(|user| {
//!         user.active.eq(true)
//!             & user.age.gte(18)
//!     })
//!     .sort_by(|user| user.name.asc())
//!     .limit(20)
//!     .all()
//!     .await?;
//! ```

mod bson_type;
mod builder;
mod expression;
mod field;
mod field_schema;
mod geospatial;
mod queryable;
mod sort;
mod text_search;
mod update_expression;

// Core typed-query API.
pub use builder::Query;
pub use expression::Expression;
pub use field::{Field, RegexOption};
pub use queryable::Queryable;
pub use sort::SortExpression;
pub use update_expression::UpdateExpression;

// BSON and text-search configuration.
pub use bson_type::BsonType;
pub use text_search::TextSearch;

// GeoJSON values and geospatial query configuration.
pub use geospatial::{GeoPoint, GeoPolygon, NearQuery};

// Internal support used by generated code and typed-field implementations.
#[doc(hidden)]
pub use expression::ElementExpression;

#[doc(hidden)]
pub use field::{
    DateQueryValue, ElementField, IntegerQueryValue, NumericQueryValue, OrderedQueryValue,
    StringQueryValue,
};

#[doc(hidden)]
pub use field_schema::FieldSchema;

#[doc(hidden)]
pub use geospatial::{GeoGeometry, GeoPointQueryValue, GeoQueryValue};
