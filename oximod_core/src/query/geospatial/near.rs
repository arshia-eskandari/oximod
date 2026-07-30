//! MongoDB `$near` query configuration.

use mongodb::bson::{Bson, Document};

use super::GeoPoint;

/// Configuration for a MongoDB `$near` query.
///
/// A [`GeoPoint`] can be passed directly to
/// [`crate::query::Field::near`] for an unrestricted proximity query.
/// `NearQuery` is used when minimum or maximum distance constraints are
/// required.
///
/// # Example
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
///             .min_distance(500.0)
///             .max_distance(5_000.0),
///         )
///     })
///     .all()
///     .await?;
/// ```
///
/// Distances are expressed in metres when querying GeoJSON values through a
/// `2dsphere` index.
///
/// OxiMod does not validate that minimum distance is less than maximum
/// distance or that either value is non-negative. MongoDB validates the final
/// query.
#[must_use]
#[derive(Debug, Clone, PartialEq)]
pub struct NearQuery {
    point: GeoPoint,
    min_distance: Option<f64>,
    max_distance: Option<f64>,
}

impl NearQuery {
    /// Creates a `$near` query around `point`.
    pub const fn new(point: GeoPoint) -> Self {
        Self {
            point,
            min_distance: None,
            max_distance: None,
        }
    }

    /// Sets the minimum distance in metres.
    pub const fn min_distance(mut self, distance: f64) -> Self {
        self.min_distance = Some(distance);
        self
    }

    /// Sets the maximum distance in metres.
    pub const fn max_distance(mut self, distance: f64) -> Self {
        self.max_distance = Some(distance);
        self
    }

    pub(super) fn into_document(self) -> Document {
        let mut document = Document::new();

        document.insert("$geometry", Bson::Document(self.point.into_document()));

        if let Some(min_distance) = self.min_distance {
            document.insert("$minDistance", min_distance);
        }

        if let Some(max_distance) = self.max_distance {
            document.insert("$maxDistance", max_distance);
        }

        document
    }
}

impl From<GeoPoint> for NearQuery {
    fn from(point: GeoPoint) -> Self {
        Self::new(point)
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::NearQuery;
    use crate::query::GeoPoint;

    #[test]
    fn near_query_builds_geometry() {
        assert_eq!(
            NearQuery::new(GeoPoint::new(0.0, 0.0),).into_document(),
            doc! {
                "$geometry": {
                    "type": "Point",
                    "coordinates": [
                        0.0,
                        0.0,
                    ],
                },
            }
        );
    }

    #[test]
    fn near_query_builds_distance_limits() {
        assert_eq!(
            NearQuery::new(GeoPoint::new(0.0, 0.0),)
                .min_distance(500.0)
                .max_distance(2_000.0)
                .into_document(),
            doc! {
                "$geometry": {
                    "type": "Point",
                    "coordinates": [
                        0.0,
                        0.0,
                    ],
                },
                "$minDistance": 500.0,
                "$maxDistance": 2_000.0,
            }
        );
    }

    #[test]
    fn point_converts_into_near_query() {
        let query: NearQuery = GeoPoint::new(1.0, 2.0).into();

        assert_eq!(
            query.into_document(),
            doc! {
                "$geometry": {
                    "type": "Point",
                    "coordinates": [
                        1.0,
                        2.0,
                    ],
                },
            }
        );
    }
}
