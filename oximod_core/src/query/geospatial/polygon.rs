//! GeoJSON polygon values.

use mongodb::bson::{Bson, Document};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A GeoJSON polygon.
///
/// [`GeoPolygon::new`] creates a polygon containing one exterior ring. The
/// ring is closed automatically when the first and last coordinates differ.
///
/// # Example
///
/// ```
/// use oximod::GeoPolygon;
///
/// let polygon = GeoPolygon::new([
///     [-1.0, -1.0],
///     [1.0, -1.0],
///     [1.0, 1.0],
///     [-1.0, 1.0],
/// ]);
/// ```
///
/// The serialized representation is equivalent to:
///
/// ```text
/// {
///     "type": "Polygon",
///     "coordinates": [[
///         [-1.0, -1.0],
///         [1.0, -1.0],
///         [1.0, 1.0],
///         [-1.0, 1.0],
///         [-1.0, -1.0]
///     ]]
/// }
/// ```
///
/// OxiMod closes the ring but does not otherwise validate polygon geometry.
/// MongoDB may reject invalid or self-intersecting polygons.
///
/// `Default` exists for compatibility with generated model builders. The
/// default polygon is empty and should be replaced before persistence.
#[must_use]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GeoPolygon {
    coordinates: Vec<Vec<[f64; 2]>>,
}

impl GeoPolygon {
    /// Creates a single-ring GeoJSON polygon.
    ///
    /// The first coordinate is appended to the end when the supplied ring is
    /// not already closed.
    ///
    /// # Parameters
    ///
    /// - `exterior`: Coordinate pairs in longitude-latitude order.
    pub fn new<I>(exterior: I) -> Self
    where
        I: IntoIterator<Item = [f64; 2]>,
    {
        let mut exterior = exterior.into_iter().collect::<Vec<_>>();

        close_ring(&mut exterior);

        Self {
            coordinates: vec![exterior],
        }
    }

    pub(super) fn into_document(self) -> Document {
        let rings = self.coordinates.into_iter().map(ring_into_bson).collect();

        let mut document = Document::new();

        document.insert("type", "Polygon");
        document.insert("coordinates", Bson::Array(rings));

        document
    }
}

impl Serialize for GeoPolygon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct GeoPolygonRef<'a> {
            #[serde(rename = "type")]
            kind: &'static str,
            coordinates: &'a [Vec<[f64; 2]>],
        }

        GeoPolygonRef {
            kind: "Polygon",
            coordinates: &self.coordinates,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GeoPolygon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct GeoPolygonDocument {
            #[serde(rename = "type")]
            kind: String,
            coordinates: Vec<Vec<[f64; 2]>>,
        }

        let polygon = GeoPolygonDocument::deserialize(deserializer)?;

        if polygon.kind != "Polygon" {
            return Err(D::Error::custom(format!(
                "expected GeoJSON type \
                     `Polygon`, found `{}`",
                polygon.kind,
            )));
        }

        Ok(Self {
            coordinates: polygon.coordinates,
        })
    }
}

fn close_ring(ring: &mut Vec<[f64; 2]>) {
    let Some(first) = ring.first().copied() else {
        return;
    };

    if ring.last().copied() != Some(first) {
        ring.push(first);
    }
}

fn ring_into_bson(ring: Vec<[f64; 2]>) -> Bson {
    let positions = ring
        .into_iter()
        .map(|[longitude, latitude]| {
            Bson::Array(vec![Bson::Double(longitude), Bson::Double(latitude)])
        })
        .collect();

    Bson::Array(positions)
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{doc, from_document, to_document};

    use super::GeoPolygon;

    #[test]
    fn polygon_closes_exterior_ring() {
        let polygon = GeoPolygon::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]);

        assert_eq!(
            to_document(&polygon).expect("polygon should serialize",),
            doc! {
                "type": "Polygon",
                "coordinates": [[
                    [-1.0, -1.0],
                    [1.0, -1.0],
                    [1.0, 1.0],
                    [-1.0, 1.0],
                    [-1.0, -1.0],
                ]],
            }
        );
    }

    #[test]
    fn polygon_preserves_existing_closure() {
        let polygon = GeoPolygon::new([
            [-1.0, -1.0],
            [1.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
            [-1.0, -1.0],
        ]);

        assert_eq!(
            to_document(&polygon).expect("polygon should serialize",),
            doc! {
                "type": "Polygon",
                "coordinates": [[
                    [-1.0, -1.0],
                    [1.0, -1.0],
                    [1.0, 1.0],
                    [-1.0, 1.0],
                    [-1.0, -1.0],
                ]],
            }
        );
    }

    #[test]
    fn polygon_deserializes_from_geojson() {
        let polygon = from_document::<GeoPolygon>(doc! {
            "type": "Polygon",
            "coordinates": [[
                [-1.0, -1.0],
                [1.0, -1.0],
                [1.0, 1.0],
                [-1.0, 1.0],
                [-1.0, -1.0],
            ]],
        })
        .expect("polygon should deserialize");

        assert_eq!(
            to_document(&polygon).expect("polygon should serialize",),
            doc! {
                "type": "Polygon",
                "coordinates": [[
                    [-1.0, -1.0],
                    [1.0, -1.0],
                    [1.0, 1.0],
                    [-1.0, 1.0],
                    [-1.0, -1.0],
                ]],
            }
        );
    }

    #[test]
    fn polygon_rejects_wrong_geojson_type() {
        let result = from_document::<GeoPolygon>(doc! {
            "type": "Point",
            "coordinates": [],
        });

        assert!(result.is_err());
    }

    #[test]
    fn polygon_default_is_builder_compatible() {
        assert_eq!(
            GeoPolygon::default(),
            GeoPolygon {
                coordinates: Vec::new(),
            },
        );
    }
}
