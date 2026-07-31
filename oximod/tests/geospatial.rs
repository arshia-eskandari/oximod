mod common;

use common::init;
use mongodb::bson::{doc, oid::ObjectId, to_document};
use oximod::{GeoPoint, GeoPolygon, Model, NearQuery, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

// Run test:
// cargo nextest run geo_point_serializes_as_geojson
#[test]
fn geo_point_serializes_as_geojson() -> TestResult {
    let point = GeoPoint::new(-79.38, 43.65);

    assert_eq!(
        to_document(&point)?,
        doc! {
            "type": "Point",
            "coordinates": [
                -79.38,
                43.65,
            ],
        }
    );

    Ok(())
}

// Run test:
// cargo nextest run geo_polygon_serializes_as_closed_geojson_ring
#[test]
fn geo_polygon_serializes_as_closed_geojson_ring() -> TestResult {
    let polygon = GeoPolygon::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]);

    assert_eq!(
        to_document(&polygon)?,
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

    Ok(())
}

// Run test:
// cargo nextest run geo_polygon_preserves_existing_closure
#[test]
fn geo_polygon_preserves_existing_closure() -> TestResult {
    let polygon = GeoPolygon::new([
        [-1.0, -1.0],
        [1.0, -1.0],
        [1.0, 1.0],
        [-1.0, 1.0],
        [-1.0, -1.0],
    ]);

    assert_eq!(
        to_document(&polygon)?,
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

    Ok(())
}

// Run test:
// cargo nextest run typed_query_near_sorts_by_distance
#[tokio::test]
async fn typed_query_near_sorts_by_distance() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_near_sorts_by_distance")]
    pub struct Place {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(geo_2dsphere)]
        location: GeoPoint,
    }

    Place::clear().await?;

    Place::default()
        .name("City1")
        .location(GeoPoint::new(0.0, 0.0))
        .save()
        .await?;

    Place::default()
        .name("City2")
        .location(GeoPoint::new(0.01, 0.0))
        .save()
        .await?;

    Place::default()
        .name("City3")
        .location(GeoPoint::new(0.03, 0.0))
        .save()
        .await?;

    let places: Vec<Place> = Place::query()
        .filter(|place| place.location.near(GeoPoint::new(0.0, 0.0)))
        .all()
        .await?;

    let names = places
        .iter()
        .map(|place| place.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["City1", "City2", "City3"],);

    Place::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_near_accepts_distance_limits
#[tokio::test]
async fn typed_query_near_accepts_distance_limits() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_near_accepts_distance_limits")]
    pub struct Place {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(geo_2dsphere)]
        location: GeoPoint,
    }

    Place::clear().await?;

    Place::default()
        .name("City1")
        .location(GeoPoint::new(0.0, 0.0))
        .save()
        .await?;

    Place::default()
        .name("City2")
        .location(GeoPoint::new(0.01, 0.0))
        .save()
        .await?;

    Place::default()
        .name("City3")
        .location(GeoPoint::new(0.03, 0.0))
        .save()
        .await?;

    let places: Vec<Place> = Place::query()
        .filter(|place| {
            place.location.near(
                NearQuery::new(GeoPoint::new(0.0, 0.0))
                    .min_distance(500.0)
                    .max_distance(2_000.0),
            )
        })
        .all()
        .await?;

    assert_eq!(places.len(), 1);
    assert_eq!(places[0].name, "City2");

    Place::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_geo_within_matches_points_inside_polygon
#[tokio::test]
async fn typed_query_geo_within_matches_points_inside_polygon() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_geo_within_matches_points_inside_polygon")]
    pub struct Place {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(geo_2dsphere)]
        location: GeoPoint,
    }

    Place::clear().await?;

    Place::default()
        .name("City1")
        .location(GeoPoint::new(0.0, 0.0))
        .save()
        .await?;

    Place::default()
        .name("City2")
        .location(GeoPoint::new(0.5, 0.5))
        .save()
        .await?;

    Place::default()
        .name("City3")
        .location(GeoPoint::new(2.0, 2.0))
        .save()
        .await?;

    let boundary = GeoPolygon::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]);

    let places: Vec<Place> = Place::query()
        .filter(|place| place.location.geo_within(boundary))
        .sort_by(|place| place.name.asc())
        .all()
        .await?;

    let names = places
        .iter()
        .map(|place| place.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["City1", "City2"]);

    Place::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_geo_intersects_matches_intersecting_polygon
#[tokio::test]
async fn typed_query_geo_intersects_matches_intersecting_polygon() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_geo_intersects_matches_intersecting_polygon")]
    pub struct Region {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(geo_2dsphere)]
        boundary: GeoPolygon,
    }

    Region::clear().await?;

    Region::default()
        .name("Region1")
        .boundary(GeoPolygon::new([
            [-1.0, -1.0],
            [1.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
        ]))
        .save()
        .await?;

    Region::default()
        .name("Region2")
        .boundary(GeoPolygon::new([
            [4.0, 4.0],
            [6.0, 4.0],
            [6.0, 6.0],
            [4.0, 6.0],
        ]))
        .save()
        .await?;

    let regions: Vec<Region> = Region::query()
        .filter(|region| region.boundary.geo_intersects(GeoPoint::new(0.25, 0.25)))
        .all()
        .await?;

    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].name, "Region1");

    Region::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_geospatial_query_combines_with_typed_filter
#[tokio::test]
async fn typed_geospatial_query_combines_with_typed_filter() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_geospatial_query_combines_with_typed_filter")]
    pub struct Place {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        active: bool,

        #[index(geo_2dsphere)]
        location: GeoPoint,
    }

    Place::clear().await?;

    Place::default()
        .name("City1")
        .active(true)
        .location(GeoPoint::new(0.0, 0.0))
        .save()
        .await?;

    Place::default()
        .name("City2")
        .active(false)
        .location(GeoPoint::new(0.5, 0.5))
        .save()
        .await?;

    Place::default()
        .name("City3")
        .active(true)
        .location(GeoPoint::new(2.0, 2.0))
        .save()
        .await?;

    let boundary = GeoPolygon::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]);

    let places: Vec<Place> = Place::query()
        .filter(|place| place.location.geo_within(boundary) & place.active.eq(true))
        .all()
        .await?;

    assert_eq!(places.len(), 1);
    assert_eq!(places[0].name, "City1");

    Place::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_geospatial_query_supports_optional_point
#[tokio::test]
async fn typed_geospatial_query_supports_optional_point() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_geospatial_query_supports_optional_point")]
    pub struct Place {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        #[index(geo_2dsphere)]
        location: Option<GeoPoint>,
    }

    Place::clear().await?;

    Place::default()
        .name("City1")
        .location(GeoPoint::new(0.0, 0.0))
        .save()
        .await?;

    Place::default().name("City2").save().await?;

    let places: Vec<Place> = Place::query()
        .filter(|place| place.location.near(GeoPoint::new(0.0, 0.0)))
        .all()
        .await?;

    assert_eq!(places.len(), 1);
    assert_eq!(places[0].name, "City1");

    Place::clear().await?;

    Ok(())
}
