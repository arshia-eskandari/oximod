//! GeoJSON values and typed MongoDB geospatial queries.
//!
//! This module provides:
//!
//! - [`GeoPoint`] for GeoJSON point values;
//! - [`GeoPolygon`] for single-ring GeoJSON polygon values;
//! - [`NearQuery`] for configuring MongoDB `$near` queries;
//! - typed [`Field`](crate::query::Field) methods for `$near`,
//!   `$geoWithin`, and `$geoIntersects`.
//!
//! GeoJSON coordinates use longitude-latitude order.
//!
//! # Example
//!
//! ```ignore
//! use oximod::{
//!     GeoPoint,
//!     NearQuery,
//!     Queryable,
//! };
//!
//! let places = Place::query()
//!     .filter(|place| {
//!         place.location.near(
//!             NearQuery::new(
//!                 GeoPoint::new(-79.38, 43.65),
//!             )
//!             .max_distance(5_000.0),
//!         )
//!     })
//!     .all()
//!     .await?;
//! ```

mod field;
mod near;
mod point;
mod polygon;
mod traits;

// Public geospatial query values.
pub use near::NearQuery;
pub use point::GeoPoint;
pub use polygon::GeoPolygon;

// Internal marker and conversion traits used by generated typed fields.
#[doc(hidden)]
pub use traits::{GeoGeometry, GeoPointQueryValue, GeoQueryValue};
