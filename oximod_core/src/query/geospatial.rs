//! GeoJSON values and typed geospatial queries.
//!
//! This module provides:
//!
//! - [`GeoPoint`] for GeoJSON point values
//! - [`GeoPolygon`] for single-ring GeoJSON polygons
//! - [`NearQuery`] for configuring MongoDB `$near` queries
//! - Typed [`Field`](crate::query::Field) methods for `$near`,
//!   `$geoWithin`, and `$geoIntersects`
//!
//! GeoJSON coordinates use longitude-latitude order.
//!
//! # Example
//!
//! ```ignore
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

pub use near::NearQuery;
pub use point::GeoPoint;
pub use polygon::GeoPolygon;

#[doc(hidden)]
pub use traits::{GeoGeometry, GeoPointQueryValue, GeoQueryValue};
