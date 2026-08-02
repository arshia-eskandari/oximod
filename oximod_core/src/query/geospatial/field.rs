//! Typed geospatial field queries.
//!
//! This module adds MongoDB `$near`, `$geoWithin`, and `$geoIntersects`
//! operations to fields containing supported GeoJSON values.

use mongodb::bson::{Bson, Document};

use super::{GeoGeometry, GeoPointQueryValue, GeoPolygon, GeoQueryValue, NearQuery};
use crate::query::expression::{ComparisonOperator, Expression};
use crate::query::field::Field;

impl<T> Field<T>
where
    T: GeoPointQueryValue,
{
    /// Creates a MongoDB `$near` query.
    ///
    /// A [`crate::query::GeoPoint`] creates a basic proximity query:
    ///
    /// ```ignore
    /// let places = Place::query()
    ///     .filter(|place| {
    ///         place.location.near(
    ///             GeoPoint::new(
    ///                 -79.38,
    ///                 43.65,
    ///             ),
    ///         )
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// Use [`NearQuery`] to configure minimum or maximum distance:
    ///
    /// ```ignore
    /// let places = Place::query()
    ///     .filter(|place| {
    ///         place.location.near(
    ///             NearQuery::new(
    ///                 GeoPoint::new(
    ///                     -79.38,
    ///                     43.65,
    ///                 ),
    ///             )
    ///             .max_distance(5_000.0),
    ///         )
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// The field must have a compatible geospatial index. MongoDB returns
    /// matching documents from nearest to farthest.
    pub fn near<N>(&self, query: N) -> Expression
    where
        N: Into<NearQuery>,
    {
        Expression::comparison(
            self.name(),
            ComparisonOperator::Near,
            Bson::Document(query.into().into_document()),
        )
    }
}

impl<T> Field<T>
where
    T: GeoQueryValue,
{
    /// Matches values contained within a GeoJSON polygon.
    ///
    /// This produces MongoDB's `$geoWithin` operator with a `$geometry`
    /// document.
    ///
    /// ```ignore
    /// let boundary = GeoPolygon::new([
    ///     [-1.0, -1.0],
    ///     [1.0, -1.0],
    ///     [1.0, 1.0],
    ///     [-1.0, 1.0],
    /// ]);
    ///
    /// let places = Place::query()
    ///     .filter(|place| {
    ///         place.location
    ///             .geo_within(boundary)
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// `$geoWithin` does not guarantee distance ordering.
    pub fn geo_within(&self, polygon: GeoPolygon) -> Expression {
        geometry_expression(self.name(), ComparisonOperator::GeoWithin, polygon)
    }

    /// Matches values that intersect the supplied GeoJSON geometry.
    ///
    /// This produces MongoDB's `$geoIntersects` operator with a `$geometry`
    /// document. Supported query geometries currently include
    /// [`crate::query::GeoPoint`] and [`GeoPolygon`].
    ///
    /// ```ignore
    /// let regions = Region::query()
    ///     .filter(|region| {
    ///         region.boundary.geo_intersects(
    ///             GeoPoint::new(0.25, 0.25),
    ///         )
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    pub fn geo_intersects<G>(&self, geometry: G) -> Expression
    where
        G: GeoGeometry,
    {
        geometry_expression(self.name(), ComparisonOperator::GeoIntersects, geometry)
    }
}

fn geometry_expression<G>(field: &str, operator: ComparisonOperator, geometry: G) -> Expression
where
    G: GeoGeometry,
{
    let mut value = Document::new();

    value.insert(
        "$geometry",
        Bson::Document(geometry.into_geometry_document()),
    );

    Expression::comparison(field, operator, Bson::Document(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{GeoPoint, GeoPolygon};
    use mongodb::bson::doc;

    #[test]
    fn point_field_builds_near_query() {
        let location = Field::<GeoPoint>::new("location");

        assert_eq!(
            location.near(GeoPoint::new(0.0, 0.0)).into_document(),
            doc! {
                "location": {
                    "$near": {
                        "$geometry": {
                            "type": "Point",
                            "coordinates": [
                                0.0,
                                0.0,
                            ],
                        },
                    },
                },
            }
        );
    }

    #[test]
    fn optional_point_field_supports_near() {
        let location = Field::<Option<GeoPoint>>::new("location");

        assert_eq!(
            location.near(GeoPoint::new(0.0, 0.0)).into_document(),
            doc! {
                "location": {
                    "$near": {
                        "$geometry": {
                            "type": "Point",
                            "coordinates": [
                                0.0,
                                0.0,
                            ],
                        },
                    },
                },
            }
        );
    }

    #[test]
    fn point_field_builds_geo_within_query() {
        let location = Field::<GeoPoint>::new("location");
        let polygon = GeoPolygon::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]);

        assert_eq!(
            location.geo_within(polygon).into_document(),
            doc! {
                "location": {
                    "$geoWithin": {
                        "$geometry": {
                            "type": "Polygon",
                            "coordinates": [[
                                [-1.0, -1.0],
                                [1.0, -1.0],
                                [1.0, 1.0],
                                [-1.0, 1.0],
                                [-1.0, -1.0],
                            ]],
                        },
                    },
                },
            }
        );
    }

    #[test]
    fn polygon_field_builds_geo_intersects_query() {
        let boundary = Field::<GeoPolygon>::new("boundary");

        assert_eq!(
            boundary
                .geo_intersects(GeoPoint::new(0.25, 0.25),)
                .into_document(),
            doc! {
                "boundary": {
                    "$geoIntersects": {
                        "$geometry": {
                            "type": "Point",
                            "coordinates": [
                                0.25,
                                0.25,
                            ],
                        },
                    },
                },
            }
        );
    }

    #[test]
    fn point_field_builds_near_query_with_distance_limits() {
        let location = Field::<GeoPoint>::new("location");

        assert_eq!(
            location
                .near(
                    NearQuery::new(GeoPoint::new(0.0, 0.0),)
                        .min_distance(500.0)
                        .max_distance(2_000.0),
                )
                .into_document(),
            doc! {
                "location": {
                    "$near": {
                        "$geometry": {
                            "type": "Point",
                            "coordinates": [
                                0.0,
                                0.0,
                            ],
                        },
                        "$minDistance": 500.0,
                        "$maxDistance": 2_000.0,
                    },
                },
            }
        );
    }
}
