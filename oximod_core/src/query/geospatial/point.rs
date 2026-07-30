//! GeoJSON point values.

use mongodb::bson::{Document, doc};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A GeoJSON point.
///
/// Coordinates are provided in longitude-latitude order.
///
/// # Example
///
/// ```
/// use oximod::GeoPoint;
///
/// let point = GeoPoint::new(
///     -79.38,
///     43.65,
/// );
///
/// assert_eq!(point.longitude(), -79.38);
/// assert_eq!(point.latitude(), 43.65);
/// ```
///
/// The serialized MongoDB representation is equivalent to:
///
/// ```text
/// {
///     "type": "Point",
///     "coordinates": [-79.38, 43.65]
/// }
/// ```
///
/// Coordinate ranges are not validated by OxiMod. MongoDB may reject invalid
/// geospatial values when they are indexed or queried.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GeoPoint {
    coordinates: [f64; 2],
}

impl GeoPoint {
    /// Creates a GeoJSON point.
    ///
    /// # Parameters
    ///
    /// - `longitude`: The east-west coordinate.
    /// - `latitude`: The north-south coordinate.
    ///
    /// Longitude must be supplied before latitude.
    pub const fn new(longitude: f64, latitude: f64) -> Self {
        Self {
            coordinates: [longitude, latitude],
        }
    }

    /// Returns the longitude coordinate.
    pub const fn longitude(self) -> f64 {
        self.coordinates[0]
    }

    /// Returns the latitude coordinate.
    pub const fn latitude(self) -> f64 {
        self.coordinates[1]
    }

    pub(super) fn into_document(self) -> Document {
        let [longitude, latitude] = self.coordinates;

        doc! {
            "type": "Point",
            "coordinates": [
                longitude,
                latitude,
            ],
        }
    }
}

impl Serialize for GeoPoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct GeoPointRef<'a> {
            #[serde(rename = "type")]
            kind: &'static str,
            coordinates: &'a [f64; 2],
        }

        GeoPointRef {
            kind: "Point",
            coordinates: &self.coordinates,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GeoPoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct GeoPointDocument {
            #[serde(rename = "type")]
            kind: String,
            coordinates: [f64; 2],
        }

        let point = GeoPointDocument::deserialize(deserializer)?;

        if point.kind != "Point" {
            return Err(D::Error::custom(format!(
                "expected GeoJSON type \
                     `Point`, found `{}`",
                point.kind,
            )));
        }

        Ok(Self {
            coordinates: point.coordinates,
        })
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{doc, from_document, to_document};

    use super::GeoPoint;

    #[test]
    fn point_serializes_as_geojson() {
        let point = GeoPoint::new(-79.38, 43.65);

        assert_eq!(
            to_document(&point).expect("point should serialize",),
            doc! {
                "type": "Point",
                "coordinates": [
                    -79.38,
                    43.65,
                ],
            }
        );
    }

    #[test]
    fn point_deserializes_from_geojson() {
        let point: GeoPoint = from_document(doc! {
            "type": "Point",
            "coordinates": [
                -79.38,
                43.65,
            ],
        })
        .expect("point should deserialize");

        assert_eq!(point, GeoPoint::new(-79.38, 43.65),);
    }

    #[test]
    fn point_rejects_wrong_geojson_type() {
        let result = from_document::<GeoPoint>(doc! {
            "type": "Polygon",
            "coordinates": [
                -79.38,
                43.65,
            ],
        });

        assert!(result.is_err());
    }

    #[test]
    fn point_default_is_origin() {
        assert_eq!(GeoPoint::default(), GeoPoint::new(0.0, 0.0),);
    }
}
