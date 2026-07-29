use mongodb::bson::{Bson, Document};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::expression::{ComparisonOperator, Expression};
use super::field::Field;

/// A GeoJSON point.
///
/// Coordinates are provided in longitude-latitude order.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GeoPoint {
    coordinates: [f64; 2],
}

impl GeoPoint {
    /// Creates a GeoJSON point.
    ///
    /// `longitude` must be provided before `latitude`.
    pub const fn new(longitude: f64, latitude: f64) -> Self {
        Self {
            coordinates: [longitude, latitude],
        }
    }

    /// Returns the longitude.
    pub const fn longitude(self) -> f64 {
        self.coordinates[0]
    }

    /// Returns the latitude.
    pub const fn latitude(self) -> f64 {
        self.coordinates[1]
    }

    fn into_document(self) -> Document {
        let [longitude, latitude] = self.coordinates;

        let mut document = Document::new();

        document.insert("type", "Point");
        document.insert(
            "coordinates",
            Bson::Array(vec![Bson::Double(longitude), Bson::Double(latitude)]),
        );

        document
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
                "expected GeoJSON type `Point`, found `{}`",
                point.kind,
            )));
        }

        Ok(Self {
            coordinates: point.coordinates,
        })
    }
}

/// A GeoJSON polygon.
///
/// `GeoPolygon::new()` creates a polygon with one exterior ring.
/// The ring is closed automatically when necessary.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GeoPolygon {
    coordinates: Vec<Vec<[f64; 2]>>,
}

impl GeoPolygon {
    /// Creates a single-ring GeoJSON polygon.
    ///
    /// The first coordinate is appended to the end when the supplied
    /// ring is not already closed.
    pub fn new<I>(exterior: I) -> Self
    where
        I: IntoIterator<Item = [f64; 2]>,
    {
        let mut exterior = exterior.into_iter().collect::<Vec<_>>();

        if let Some(first) = exterior.first().copied()
            && exterior.last().copied() != Some(first)
        {
            exterior.push(first);
        }

        Self {
            coordinates: vec![exterior],
        }
    }

    fn into_document(self) -> Document {
        let rings = self
            .coordinates
            .into_iter()
            .map(|ring| {
                let positions = ring
                    .into_iter()
                    .map(|[longitude, latitude]| {
                        Bson::Array(vec![Bson::Double(longitude), Bson::Double(latitude)])
                    })
                    .collect();

                Bson::Array(positions)
            })
            .collect();

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
                "expected GeoJSON type `Polygon`, found `{}`",
                polygon.kind,
            )));
        }

        Ok(Self {
            coordinates: polygon.coordinates,
        })
    }
}

/// Configuration for a MongoDB `$near` query.
#[derive(Debug, Clone, PartialEq)]
pub struct NearQuery {
    point: GeoPoint,
    min_distance: Option<f64>,
    max_distance: Option<f64>,
}

impl NearQuery {
    /// Creates a `$near` query around a GeoJSON point.
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

    fn into_document(self) -> Document {
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

/// A GeoJSON value that can be supplied to a geospatial query.
#[doc(hidden)]
pub trait GeoGeometry {
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

/// Marker for fields containing supported GeoJSON values.
#[doc(hidden)]
pub trait GeoQueryValue {}

impl GeoQueryValue for GeoPoint {}
impl GeoQueryValue for Option<GeoPoint> {}
impl GeoQueryValue for GeoPolygon {}
impl GeoQueryValue for Option<GeoPolygon> {}

/// Marker for fields that can use `$near`.
#[doc(hidden)]
pub trait GeoPointQueryValue: GeoQueryValue {}

impl GeoPointQueryValue for GeoPoint {}
impl GeoPointQueryValue for Option<GeoPoint> {}

impl<T> Field<T>
where
    T: GeoPointQueryValue,
{
    /// Creates a MongoDB `$near` expression.
    ///
    /// A [`GeoPoint`] creates a basic proximity query. Use
    /// [`NearQuery`] to configure minimum or maximum distance.
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
    /// Creates a MongoDB `$geoWithin` expression using a GeoJSON
    /// polygon.
    pub fn geo_within(&self, polygon: GeoPolygon) -> Expression {
        geometry_expression(self.name(), ComparisonOperator::GeoWithin, polygon)
    }

    /// Creates a MongoDB `$geoIntersects` expression.
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
    use mongodb::bson::{doc, to_document};

    use super::{GeoPoint, GeoPolygon, NearQuery};
    use crate::query::Field;

    #[test]
    fn point_serializes_as_geojson() {
        let point = GeoPoint::new(-79.38, 43.65);

        assert_eq!(
            to_document(&point).expect("point should serialize"),
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
    fn polygon_closes_exterior_ring() {
        let polygon = GeoPolygon::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]);

        assert_eq!(
            to_document(&polygon).expect("polygon should serialize"),
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
    fn polygon_does_not_duplicate_existing_closure() {
        let polygon = GeoPolygon::new([
            [-1.0, -1.0],
            [1.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
            [-1.0, -1.0],
        ]);

        assert_eq!(
            to_document(&polygon).expect("polygon should serialize"),
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
    fn near_query_builds_geometry_document() {
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
    fn near_query_builds_distance_limits() {
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

    #[test]
    fn optional_point_field_supports_near_query() {
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
    fn point_field_builds_geo_within_expression() {
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
    fn polygon_field_builds_geo_intersects_expression() {
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
    fn geo_point_has_builder_compatible_default() {
        assert_eq!(GeoPoint::default(), GeoPoint::new(0.0, 0.0),);
    }

    #[test]
    fn geo_polygon_has_builder_compatible_default() {
        assert_eq!(
            GeoPolygon::default(),
            GeoPolygon {
                coordinates: Vec::new(),
            },
        );
    }
}
