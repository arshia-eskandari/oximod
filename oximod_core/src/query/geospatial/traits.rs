//! Internal geospatial marker traits.

use mongodb::bson::Document;

use super::{GeoPoint, GeoPolygon};

/// A GeoJSON value that can be supplied as query geometry.
///
/// This trait is used by typed `$geoIntersects` and related query helpers.
#[doc(hidden)]
pub trait GeoGeometry {
    /// Converts this value into a GeoJSON BSON document.
    fn into_geometry_document(self) -> Document;
}

impl GeoGeometry for GeoPoint {
    fn into_geometry_document(self) -> Document {
        self.into_document()
    }
}

impl GeoGeometry for GeoPolygon {
    fn into_geometry_document(self) -> Document {
        self.into_document()
    }
}

/// Marks fields containing a supported GeoJSON value.
#[doc(hidden)]
pub trait GeoQueryValue {}

impl GeoQueryValue for GeoPoint {}
impl GeoQueryValue for Option<GeoPoint> {}
impl GeoQueryValue for GeoPolygon {}
impl GeoQueryValue for Option<GeoPolygon> {}

/// Marks GeoJSON point fields that support MongoDB `$near`.
#[doc(hidden)]
pub trait GeoPointQueryValue: GeoQueryValue {}

impl GeoPointQueryValue for GeoPoint {}
impl GeoPointQueryValue for Option<GeoPoint> {}
