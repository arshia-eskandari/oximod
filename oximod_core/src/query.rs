//! Type-safe MongoDB queries and updates.
//!
//! This module contains the building blocks used by OxiMod's typed query API.
//! Most applications begin with [`Queryable::query`] and construct filters,
//! sorting, updates, text searches, and geospatial queries through generated
//! [`Field`] values.
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
//!
//! Public query types are re-exported from the `oximod` crate root, so
//! applications normally import them directly from `oximod`.

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

// Core query API.
pub use builder::Query;
pub use expression::Expression;
pub use field::{Field, RegexOption};
pub use field_schema::FieldSchema;
pub use queryable::Queryable;
pub use sort::SortExpression;
pub use update_expression::UpdateExpression;

// BSON and text-search configuration.
pub use bson_type::BsonType;
pub use text_search::TextSearch;

// GeoJSON values and geospatial query configuration.
pub use geospatial::{GeoPoint, GeoPolygon, NearQuery};

// Internal support types used by generated code and typed field methods.
#[doc(hidden)]
pub use expression::ElementExpression;

#[doc(hidden)]
pub use field::{
    DateQueryValue, ElementField, IntegerQueryValue, NumericQueryValue, OrderedQueryValue,
    StringQueryValue,
};

#[doc(hidden)]
pub use geospatial::{GeoGeometry, GeoPointQueryValue, GeoQueryValue};
