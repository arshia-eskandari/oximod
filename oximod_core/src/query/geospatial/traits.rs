//! Internal geospatial value categories and geometry conversion.
//!
//! These traits connect supported GeoJSON values to OxiMod's typed
//! geospatial field methods. They are hidden implementation details and are
//! not intended to be implemented or referenced by applications.

use mongodb::bson::Document;

use super::{GeoPoint, GeoPolygon};

/// A GeoJSON value that can be supplied as query geometry.
///
/// Implementations can be passed to typed helpers such as
/// [`crate::query::Field::geo_intersects`].
#[doc(hidden)]
pub trait GeoGeometry {
    /// Converts this value into its GeoJSON BSON document.
    #[must_use]
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

/// Marks required and optional fields containing a supported GeoJSON value.
///
/// Implementations receive typed `$geoWithin` and `$geoIntersects` field
/// methods.
#[doc(hidden)]
pub trait GeoQueryValue {}

impl GeoQueryValue for GeoPoint {}
impl GeoQueryValue for Option<GeoPoint> {}
impl GeoQueryValue for GeoPolygon {}
impl GeoQueryValue for Option<GeoPolygon> {}

/// Marks required and optional GeoJSON point fields that support MongoDB
/// `$near`.
///
/// Polygon fields intentionally do not implement this trait.
#[doc(hidden)]
pub trait GeoPointQueryValue: GeoQueryValue {}

impl GeoPointQueryValue for GeoPoint {}
impl GeoPointQueryValue for Option<GeoPoint> {}

#[cfg(test)]
mod tests {
    use super::{GeoGeometry, GeoPoint, GeoPointQueryValue, GeoPolygon, GeoQueryValue};
    use mongodb::bson::doc;

    fn assert_geo_query_value<T: GeoQueryValue>() {}

    fn assert_geo_point_query_value<T: GeoPointQueryValue>() {}

    #[test]
    fn point_geometry_converts_into_geojson_document() {
        assert_eq!(
            GeoPoint::new(-79.38, 43.65).into_geometry_document(),
            doc! {
                "type": "Point",
                "coordinates": [
                    -79.38,
                    43.65,
                ],
            },
        );
    }

    #[test]
    fn polygon_geometry_converts_into_geojson_document() {
        assert_eq!(
            GeoPolygon::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0],])
                .into_geometry_document(),
            doc! {
                "type": "Polygon",
                "coordinates": [[
                    [-1.0, -1.0],
                    [1.0, -1.0],
                    [1.0, 1.0],
                    [-1.0, 1.0],
                    [-1.0, -1.0],
                ]],
            },
        );
    }

    #[test]
    fn geospatial_query_values_are_registered() {
        assert_geo_query_value::<GeoPoint>();
        assert_geo_query_value::<Option<GeoPoint>>();
        assert_geo_query_value::<GeoPolygon>();
        assert_geo_query_value::<Option<GeoPolygon>>();
    }

    #[test]
    fn near_query_values_are_registered() {
        assert_geo_point_query_value::<GeoPoint>();
        assert_geo_point_query_value::<Option<GeoPoint>>();
    }
}
