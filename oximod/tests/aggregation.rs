//! Integration tests for OxiMod's first-class aggregation API.
//!
//! Most tests use an explicit `mongodb::Client` created from `MONGODB_URI`,
//! so aggregation is proven independent of the process-global client. The
//! audit-derived workflows (shape-preserving pipelines, `$geoNear` distance
//! output, device/time-window rollups, text relevance scores) are each
//! checked against an independent raw-driver oracle pipeline.

use futures_util::TryStreamExt;
use mongodb::{
    Client,
    bson::{DateTime, doc, oid::ObjectId},
    options::AggregateOptions,
};
use oximod::{GeoPoint, Model, OxiModError, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;

/// Unreachable deployment with a short server-selection timeout.
const UNREACHABLE_URI: &str =
    "mongodb://127.0.0.1:1/?serverSelectionTimeoutMS=300&connectTimeoutMS=300";

async fn client() -> Result<Client, Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
    Ok(Client::with_uri_str(uri).await?)
}

fn driver_source(error: &OxiModError) -> Option<&mongodb::error::Error> {
    std::error::Error::source(error)
        .and_then(|source| source.downcast_ref::<mongodb::error::Error>())
}

// --- E1: shape-preserving typed pipeline vs raw oracle ---------------------

// Run test: cargo nextest run typed_pipeline_matches_raw_oracle_with_serde_rename
#[tokio::test]
async fn typed_pipeline_matches_raw_oracle_with_serde_rename() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("typed_pipeline_rename")]
    pub struct Employee {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[serde(rename = "fullName")]
        name: String,

        age: i32,
        active: bool,
    }

    let client = client().await?;
    Employee::clear_from(&client).await?;

    let seed = [
        ("Employee1", 42, true),
        ("Employee2", 35, true),
        ("Employee3", 35, false),
        ("Employee4", 30, true),
        ("Employee5", 51, true),
        ("Employee6", 28, true),
    ];

    for (name, age, active) in seed {
        Employee::new()
            .name(name)
            .age(age)
            .active(active)
            .save_from(&client)
            .await?;
    }

    let raw_collection = Employee::get_document_collection_from(&client)?;

    let before: Vec<_> = raw_collection
        .find(doc! {})
        .sort(doc! { "_id": 1 })
        .await?
        .try_collect()
        .await?;

    // The typed arm exercises match, a multi-key single $sort stage, and a
    // $limit stage, using the Rust field name for the renamed field.
    let typed: Vec<Employee> = Employee::aggregate()
        .match_(|employee| employee.active.eq(true) & employee.age.gte(30))
        .sort_by(|employee| employee.age.desc().then(employee.name.asc()))
        .limit(3)
        .all_from(&client)
        .await?;

    // Independent raw-driver oracle with hand-written serialized field names.
    let oracle: Vec<_> = raw_collection
        .aggregate([
            doc! {
                "$match": {
                    "$and": [
                        { "active": true },
                        { "age": { "$gte": 30 } },
                    ],
                },
            },
            doc! { "$sort": { "age": -1, "fullName": 1 } },
            doc! { "$limit": 3 },
        ])
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    assert_eq!(typed.len(), 3);
    assert_eq!(oracle.len(), 3);

    let typed_rows: Vec<(Option<ObjectId>, &str, i32)> = typed
        .iter()
        .map(|employee| (employee._id, employee.name.as_str(), employee.age))
        .collect();

    let oracle_rows: Vec<(Option<ObjectId>, &str, i32)> = oracle
        .iter()
        .map(|row| {
            (
                row.get_object_id("_id").ok(),
                row.get_str("fullName")
                    .expect("oracle row should keep fullName"),
                row.get_i32("age").expect("oracle row should keep age"),
            )
        })
        .collect();

    assert_eq!(
        typed_rows, oracle_rows,
        "typed aggregation should match the raw oracle exactly, in order"
    );

    let expected_names: Vec<&str> = vec!["Employee5", "Employee1", "Employee2"];
    let names: Vec<&str> = typed
        .iter()
        .map(|employee| employee.name.as_str())
        .collect();
    assert_eq!(names, expected_names);

    // Aggregation must not mutate the source collection.
    let after: Vec<_> = raw_collection
        .find(doc! {})
        .sort(doc! { "_id": 1 })
        .await?
        .try_collect()
        .await?;
    assert_eq!(
        before, after,
        "aggregation should not modify source documents"
    );

    Employee::clear_from(&client).await?;
    Ok(())
}

// --- Global-client execution ----------------------------------------------

// Run test: cargo nextest run global_all_and_first_execute_typed_pipeline
#[tokio::test]
async fn global_all_and_first_execute_typed_pipeline() -> TestResult {
    common::init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("global_terminals")]
    pub struct Task {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        title: String,
        priority: i32,
    }

    Task::clear().await?;

    for (title, priority) in [("Task1", 3), ("Task2", 1), ("Task3", 2)] {
        Task::new().title(title).priority(priority).save().await?;
    }

    let ordered: Vec<Task> = Task::aggregate()
        .sort_by(|task| task.priority.asc())
        .all()
        .await?;

    let titles: Vec<&str> = ordered.iter().map(|task| task.title.as_str()).collect();
    assert_eq!(titles, ["Task2", "Task3", "Task1"]);

    // first() consumes at most one result from the pipeline as built.
    let top = Task::aggregate()
        .sort_by(|task| task.priority.asc())
        .limit(1)
        .first()
        .await?
        .expect("a first task should exist");
    assert_eq!(top.title, "Task2");

    // An empty result produces None rather than an error.
    let none = Task::aggregate()
        .match_(|task| task.priority.gte(100))
        .first()
        .await?;
    assert!(none.is_none());

    Task::clear().await?;
    Ok(())
}

// Run test: cargo nextest run options_pass_through_and_multi_batch_cursor
#[tokio::test]
async fn options_pass_through_and_multi_batch_cursor() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("options_multi_batch")]
    pub struct Item {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        label: String,
        rank: i32,
    }

    let client = client().await?;
    Item::clear_from(&client).await?;

    for rank in 1..=5 {
        Item::new()
            .label(format!("Item{rank}"))
            .rank(rank)
            .save_from(&client)
            .await?;
    }

    // A batch size of 1 forces the cursor through multiple server batches.
    let options = AggregateOptions::builder()
        .batch_size(1_u32)
        .allow_disk_use(true)
        .build();

    let items: Vec<Item> = Item::aggregate()
        .sort_by(|item| item.rank.asc())
        .with_options(options)
        .all_from(&client)
        .await?;

    let ranks: Vec<i32> = items.iter().map(|item| item.rank).collect();
    assert_eq!(
        ranks,
        [1, 2, 3, 4, 5],
        "all results should arrive in order across cursor batches"
    );

    Item::clear_from(&client).await?;
    Ok(())
}

// --- E2: A5-style $geoNear distance output --------------------------------

// Run test: cargo nextest run geonear_distance_output_matches_raw_oracle
#[tokio::test]
async fn geonear_distance_output_matches_raw_oracle() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("geonear_distance")]
    pub struct Provider {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(geo_2dsphere)]
        location: GeoPoint,
    }

    /// The pipeline output: the source fields plus the server-computed
    /// distance added by `$geoNear`.
    #[derive(Debug, Deserialize)]
    struct ProviderWithDistance {
        _id: ObjectId,
        name: String,
        distance_m: f64,
    }

    let client = client().await?;
    Provider::clear_from(&client).await?;
    Provider::init_indexes_from(&client).await?;

    // Deterministic points along the equator. One degree of longitude at the
    // equator is roughly 111.32 km, so the 3.5 km radius keeps the first
    // three providers and excludes the fourth.
    let seed = [
        ("Provider1", 0.01),
        ("Provider2", 0.02),
        ("Provider3", 0.03),
        ("Provider4", 0.05),
    ];

    for (name, longitude) in seed {
        Provider::new()
            .name(name)
            .location(GeoPoint::new(longitude, 0.0))
            .save_from(&client)
            .await?;
    }

    let geo_near = doc! {
        "$geoNear": {
            "near": { "type": "Point", "coordinates": [0.0, 0.0] },
            "distanceField": "distance_m",
            "maxDistance": 3500.0,
            "spherical": true,
            "key": "location",
        },
    };

    // $geoNear must be the first stage; the typed $limit afterward operates
    // on documents that still carry the source fields.
    let typed: Vec<ProviderWithDistance> = Provider::aggregate()
        .raw_stage(geo_near.clone())
        .limit(10)
        .with_type::<ProviderWithDistance>()
        .all_from(&client)
        .await?;

    let oracle: Vec<_> = Provider::get_document_collection_from(&client)?
        .aggregate([geo_near, doc! { "$limit": 10 }])
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    assert_eq!(
        typed.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["Provider1", "Provider2", "Provider3"],
        "only providers inside the radius should qualify, nearest first"
    );

    assert_eq!(typed.len(), oracle.len());

    for (typed_row, oracle_row) in typed.iter().zip(&oracle) {
        assert_eq!(Ok(typed_row._id), oracle_row.get_object_id("_id"));

        let oracle_distance = oracle_row
            .get_f64("distance_m")
            .expect("oracle row should carry the computed distance");

        assert!(
            (typed_row.distance_m - oracle_distance).abs() < 1e-9,
            "distances should match the oracle: {} vs {oracle_distance}",
            typed_row.distance_m
        );
    }

    // Distances are ascending and within the requested radius.
    for window in typed.windows(2) {
        assert!(window[0].distance_m <= window[1].distance_m);
    }
    assert!(
        typed
            .iter()
            .all(|p| p.distance_m > 0.0 && p.distance_m <= 3500.0)
    );

    Provider::clear_from(&client).await?;
    Ok(())
}

// --- E3: A6-style device/time-window rollup --------------------------------

// Run test: cargo nextest run device_window_rollup_matches_raw_and_arithmetic_oracles
#[tokio::test]
async fn device_window_rollup_matches_raw_and_arithmetic_oracles() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("device_window_rollup")]
    pub struct Reading {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        device: String,

        #[serde(rename = "recordedAt")]
        recorded_at: Option<DateTime>,

        value: f64,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct WindowKey {
        device: String,
        hour: DateTime,
    }

    #[derive(Debug, Deserialize)]
    struct DeviceWindowRollup {
        #[serde(rename = "_id")]
        key: WindowKey,
        total: f64,
        count: i32,
    }

    let client = client().await?;
    Reading::clear_from(&client).await?;

    // 2024-01-01T00:00:00Z, in milliseconds since the epoch.
    const BASE_MS: i64 = 1_704_067_200_000;
    const HOUR_MS: i64 = 3_600_000;

    let minutes = |m: i64| DateTime::from_millis(BASE_MS + m * 60_000);

    let seed = [
        ("sensor-a", 5, 1.5),
        ("sensor-a", 20, 2.5),
        ("sensor-a", 70, 3.0),
        ("sensor-b", 30, 10.0),
        ("sensor-b", 105, 20.0),
        ("sensor-b", 110, 0.5),
        // Outside the two-hour window; the typed $match must exclude it.
        ("sensor-b", 150, 99.0),
    ];

    for (device, minute, value) in seed {
        Reading::new()
            .device(device)
            .recorded_at(minutes(minute))
            .value(value)
            .save_from(&client)
            .await?;
    }

    let window_start = DateTime::from_millis(BASE_MS);
    let window_end = DateTime::from_millis(BASE_MS + 2 * HOUR_MS);

    let typed: Vec<DeviceWindowRollup> = Reading::aggregate()
        .match_(|reading| {
            reading.recorded_at.gte(window_start) & reading.recorded_at.lt(window_end)
        })
        .raw_stage_with(|reading| {
            doc! {
                "$group": {
                    "_id": {
                        "device": format!("${}", reading.device.name()),
                        "hour": {
                            "$dateTrunc": {
                                "date": format!("${}", reading.recorded_at.name()),
                                "unit": "hour",
                            },
                        },
                    },
                    "total": { "$sum": format!("${}", reading.value.name()) },
                    "count": { "$sum": 1 },
                },
            }
        })
        .raw_stage(doc! { "$sort": { "_id.device": 1, "_id.hour": 1 } })
        .with_type::<DeviceWindowRollup>()
        .all_from(&client)
        .await?;

    // Raw-driver oracle with hand-written serialized field names.
    let oracle: Vec<_> = Reading::get_document_collection_from(&client)?
        .aggregate([
            doc! {
                "$match": {
                    "$and": [
                        { "recordedAt": { "$gte": window_start } },
                        { "recordedAt": { "$lt": window_end } },
                    ],
                },
            },
            doc! {
                "$group": {
                    "_id": {
                        "device": "$device",
                        "hour": {
                            "$dateTrunc": { "date": "$recordedAt", "unit": "hour" },
                        },
                    },
                    "total": { "$sum": "$value" },
                    "count": { "$sum": 1 },
                },
            },
            doc! { "$sort": { "_id.device": 1, "_id.hour": 1 } },
        ])
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    // Closed-form arithmetic oracle over the deterministic seed.
    let expected = [
        ("sensor-a", 0, 4.0, 2),
        ("sensor-a", 1, 3.0, 1),
        ("sensor-b", 0, 10.0, 1),
        ("sensor-b", 1, 20.5, 2),
    ];

    assert_eq!(typed.len(), expected.len(), "rollup row count should match");
    assert_eq!(oracle.len(), expected.len());

    for ((row, oracle_row), (device, hour, total, count)) in typed.iter().zip(&oracle).zip(expected)
    {
        assert_eq!(row.key.device, device);
        assert_eq!(
            row.key.hour,
            DateTime::from_millis(BASE_MS + hour * HOUR_MS)
        );
        assert!(
            (row.total - total).abs() < 1e-9,
            "total for {device}/{hour} should be {total}, got {}",
            row.total
        );
        assert_eq!(row.count, count);

        let oracle_key = oracle_row.get_document("_id").expect("oracle row key");
        assert_eq!(oracle_key.get_str("device"), Ok(device));
        assert_eq!(
            oracle_key.get_datetime("hour"),
            Ok(&DateTime::from_millis(BASE_MS + hour * HOUR_MS)),
        );
        assert!((oracle_row.get_f64("total").expect("oracle total") - total).abs() < 1e-9);
        assert_eq!(oracle_row.get_i32("count"), Ok(count));
    }

    Reading::clear_from(&client).await?;
    Ok(())
}

// --- E4: A4-style text relevance score as application data ------------------

// Run test: cargo nextest run text_relevance_score_output_matches_raw_oracle
#[tokio::test]
async fn text_relevance_score_output_matches_raw_oracle() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("text_relevance_score")]
    pub struct Article {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(text)]
        title: String,
    }

    #[derive(Debug, Deserialize)]
    struct ScoredArticle {
        _id: ObjectId,
        title: String,
        score: f64,
    }

    let client = client().await?;
    Article::clear_from(&client).await?;
    Article::init_indexes_from(&client).await?;

    for title in [
        "rust mongodb aggregation",
        "rust rust rust",
        "python tutorial",
        "mongodb basics",
    ] {
        Article::new().title(title).save_from(&client).await?;
    }

    // The $text match must be the first stage; the relevance score becomes
    // ordinary application data through $addFields + $meta.
    let typed: Vec<ScoredArticle> = Article::aggregate()
        .text("rust")
        .raw_stage(doc! {
            "$addFields": { "score": { "$meta": "textScore" } },
        })
        .raw_stage(doc! { "$sort": { "score": -1, "_id": 1 } })
        .with_type::<ScoredArticle>()
        .all_from(&client)
        .await?;

    let oracle: Vec<_> = Article::get_document_collection_from(&client)?
        .aggregate([
            doc! { "$match": { "$text": { "$search": "rust" } } },
            doc! { "$addFields": { "score": { "$meta": "textScore" } } },
            doc! { "$sort": { "score": -1, "_id": 1 } },
        ])
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    assert_eq!(typed.len(), 2, "only the rust articles should match");
    assert_eq!(typed.len(), oracle.len());

    // The heavily repeated term must rank first.
    assert_eq!(typed[0].title, "rust rust rust");

    for (typed_row, oracle_row) in typed.iter().zip(&oracle) {
        assert_eq!(Ok(typed_row._id), oracle_row.get_object_id("_id"));
        assert_eq!(Ok(typed_row.title.as_str()), oracle_row.get_str("title"));

        let oracle_score = oracle_row.get_f64("score").expect("oracle score");
        assert!(typed_row.score > 0.0);
        assert!((typed_row.score - oracle_score).abs() < 1e-9);
    }

    Article::clear_from(&client).await?;
    Ok(())
}

// --- E5: session and transaction visibility ---------------------------------

// Run test: cargo nextest run session_aggregation_sees_uncommitted_transaction_writes
#[tokio::test]
async fn session_aggregation_sees_uncommitted_transaction_writes() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("session_visibility")]
    pub struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        sku: String,
        status: String,
    }

    let client = client().await?;
    Order::clear_from(&client).await?;

    // A committed baseline document, outside any transaction.
    Order::new()
        .sku("sku-committed")
        .status("paid")
        .save_from(&client)
        .await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    Order::new()
        .sku("sku-uncommitted")
        .status("paid")
        .save_with_session(&mut session)
        .await?;

    // The same session observes both paid orders, including the uncommitted
    // one.
    let inside: Vec<Order> = Order::aggregate()
        .match_(|order| order.status.eq("paid"))
        .sort_by(|order| order.sku.asc())
        .all_with_session(&mut session)
        .await?;
    assert_eq!(
        inside.iter().map(|o| o.sku.as_str()).collect::<Vec<_>>(),
        ["sku-committed", "sku-uncommitted"],
        "session aggregation should see the transaction's own write"
    );

    let first_inside = Order::aggregate()
        .match_(|order| order.status.eq("paid"))
        .sort_by(|order| order.sku.desc())
        .first_with_session(&mut session)
        .await?
        .expect("the session should see a paid order");
    assert_eq!(first_inside.sku, "sku-uncommitted");

    // A sessionless aggregation must not observe the uncommitted write.
    let outside: Vec<Order> = Order::aggregate()
        .match_(|order| order.status.eq("paid"))
        .all_from(&client)
        .await?;
    assert_eq!(
        outside.iter().map(|o| o.sku.as_str()).collect::<Vec<_>>(),
        ["sku-committed"],
        "sessionless aggregation should not see uncommitted writes"
    );

    session.abort_transaction().await?;

    // After the abort, the uncommitted order is gone for everyone.
    let after_abort: Vec<Order> = Order::aggregate()
        .match_(|order| order.status.eq("paid"))
        .all_from(&client)
        .await?;
    assert_eq!(
        after_abort
            .iter()
            .map(|o| o.sku.as_str())
            .collect::<Vec<_>>(),
        ["sku-committed"],
        "aborted writes should not appear in later aggregations"
    );

    Order::clear_from(&client).await?;
    Ok(())
}

// --- E6: error classification ----------------------------------------------

// Run test: cargo nextest run invalid_pipeline_classifies_as_aggregation_with_driver_source
#[tokio::test]
async fn invalid_pipeline_classifies_as_aggregation_with_driver_source() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("invalid_pipeline")]
    pub struct Record {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
    }

    let client = client().await?;
    Record::clear_from(&client).await?;
    Record::new().name("Record1").save_from(&client).await?;

    let error = Record::aggregate()
        .raw_stage(doc! { "$thisStageDoesNotExist": 1 })
        .all_from(&client)
        .await
        .expect_err("an unknown stage should be rejected by the server");

    assert!(
        matches!(error, OxiModError::Aggregation { .. }),
        "a server-rejected pipeline should classify as Aggregation, got: {error:?}"
    );

    let source = driver_source(&error)
        .expect("the original driver error should be reachable through source()");
    assert!(
        matches!(*source.kind, mongodb::error::ErrorKind::Command(_)),
        "the preserved driver error should be the server command rejection, got: {source:?}"
    );

    Record::clear_from(&client).await?;
    Ok(())
}

// Run test: cargo nextest run output_type_mismatch_classifies_as_serialization
#[tokio::test]
async fn output_type_mismatch_classifies_as_serialization() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("output_type_mismatch")]
    pub struct Metric {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        value: i32,
    }

    /// Deliberately wrong: `value` is stored as an integer.
    #[derive(Debug, Deserialize)]
    struct WrongShape {
        #[allow(dead_code)]
        name: String,
        #[allow(dead_code)]
        value: String,
    }

    let client = client().await?;
    Metric::clear_from(&client).await?;

    for (name, value) in [("Metric1", 1), ("Metric2", 2)] {
        Metric::new()
            .name(name)
            .value(value)
            .save_from(&client)
            .await?;
    }

    // all(): one undecodable output document fails the whole call.
    let error = Metric::aggregate()
        .with_type::<WrongShape>()
        .all_from(&client)
        .await
        .expect_err("a mismatched output type should fail deserialization");

    assert!(
        matches!(error, OxiModError::Serialization { .. }),
        "an output decode mismatch should classify as Serialization, got: {error:?}"
    );

    // first(): the first result failing to decode is an error, not None.
    let error = Metric::aggregate()
        .with_type::<WrongShape>()
        .first_from(&client)
        .await
        .expect_err("a mismatched first result should fail deserialization");

    assert!(
        matches!(error, OxiModError::Serialization { .. }),
        "a first-result decode mismatch should classify as Serialization, got: {error:?}"
    );

    Metric::clear_from(&client).await?;
    Ok(())
}

// Run test: cargo nextest run unreachable_server_aggregation_classifies_as_connection
#[tokio::test]
async fn unreachable_server_aggregation_classifies_as_connection() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("aggregation_test")]
    #[collection("unreachable_aggregation")]
    pub struct Unreachable {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
    }

    let client = Client::with_uri_str(UNREACHABLE_URI).await?;

    let error = Unreachable::aggregate()
        .match_(|record| record.name.eq("missing"))
        .all_from(&client)
        .await
        .expect_err("aggregation should fail against an unreachable server");

    assert!(
        matches!(error, OxiModError::Connection { .. }),
        "connectivity failures outrank the aggregation domain, got: {error:?}"
    );
    assert!(
        driver_source(&error).is_some(),
        "the driver error should remain reachable through source()"
    );

    let error = Unreachable::aggregate()
        .first_from(&client)
        .await
        .expect_err("first_from should fail against an unreachable server");

    assert!(
        matches!(error, OxiModError::Connection { .. }),
        "connectivity failures outrank the aggregation domain, got: {error:?}"
    );

    Ok(())
}
