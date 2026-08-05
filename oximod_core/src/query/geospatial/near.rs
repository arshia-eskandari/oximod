//! MongoDB `$near` query configuration.
//!
//! [`NearQuery`] wraps a GeoJSON point and optional minimum and maximum
//! distances for use with [`crate::query::Field::near`].

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
/// use oximod::{
///     GeoPoint,
///     NearQuery,
///     Queryable,
/// };
///
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
/// Calling [`NearQuery::min_distance`] or [`NearQuery::max_distance`] more
/// than once replaces the previous value.
///
/// OxiMod does not validate that either distance is non-negative or that the
/// minimum distance does not exceed the maximum distance. MongoDB validates
/// the final query.
#[derive(Debug, Clone, PartialEq)]
#[must_use = "a near query must be passed to Field::near"]
pub struct NearQuery {
    point: GeoPoint,
    min_distance: Option<f64>,
    max_distance: Option<f64>,
}

impl NearQuery {
    /// Creates an unrestricted `$near` query around `point`.
    pub const fn new(point: GeoPoint) -> Self {
        Self {
            point,
            min_distance: None,
            max_distance: None,
        }
    }

    /// Sets the minimum distance in metres.
    ///
    /// Calling this method again replaces the previous minimum distance.
    pub const fn min_distance(mut self, distance: f64) -> Self {
        self.min_distance = Some(distance);
        self
    }

    /// Sets the maximum distance in metres.
    ///
    /// Calling this method again replaces the previous maximum distance.
    pub const fn max_distance(mut self, distance: f64) -> Self {
        self.max_distance = Some(distance);
        self
    }

    #[must_use]
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
    use super::NearQuery;
    use crate::query::GeoPoint;
    use mongodb::bson::doc;

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
    fn repeated_distance_setters_replace_previous_values() {
        assert_eq!(
            NearQuery::new(GeoPoint::new(0.0, 0.0),)
                .min_distance(100.0)
                .min_distance(500.0)
                .max_distance(1_000.0)
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
