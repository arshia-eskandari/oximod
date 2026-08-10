//! Failure-class contract: global-client operations against an unreachable
//! deployment (SR-2).
//!
//! This binary initializes the process-wide global client against an
//! unreachable deployment, a state that is incompatible with every test that
//! needs a working global client. It therefore lives in its own
//! integration-test binary, which keeps it deterministic under ordinary
//! `cargo test` (one process per binary) as well as under nextest's
//! process-per-test model. Any test added to this binary shares the
//! unreachable global client.
//!
//! This binary does not require `MONGODB_URI`.

use std::error::Error as StdError;

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiClient, OxiModError, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

/// Unreachable deployment with a short server-selection timeout.
const UNREACHABLE_URI: &str =
    "mongodb://127.0.0.1:1/?serverSelectionTimeoutMS=300&connectTimeoutMS=300";

/// Returns the preserved `mongodb::error::Error` behind an `OxiModError`.
fn driver_source(error: &OxiModError) -> Option<&mongodb::error::Error> {
    error
        .source()
        .and_then(|source| source.downcast_ref::<mongodb::error::Error>())
}

/// Asserts that a driver-backed error is `Connection` and keeps its source.
fn assert_connection(operation: &str, error: &OxiModError) {
    assert!(
        matches!(error, OxiModError::Connection { .. }),
        "expected {operation} against an unreachable server to classify as \
         Connection, got: {error:?}"
    );
    assert!(
        driver_source(error).is_some(),
        "expected {operation} to preserve the mongodb driver error through \
         source(), got: {error:?}"
    );
}

// Run test: cargo nextest run unreachable_server_typed_queries_classify_as_connection
#[tokio::test]
async fn unreachable_server_typed_queries_classify_as_connection() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("error_class_unreachable_typed")]
    pub struct Unreachable {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        active: bool,
    }

    OxiClient::init_global(UNREACHABLE_URI.to_string()).await?;

    let error = Unreachable::default()
        .name("User1")
        .save()
        .await
        .expect_err("save should fail against an unreachable server");
    assert_connection("save", &error);

    let error = Unreachable::query()
        .filter(|user| user.name.eq("User1"))
        .first()
        .await
        .expect_err("query().first() should fail against an unreachable server");
    assert_connection("query().first()", &error);

    let error = Unreachable::query()
        .filter(|user| user.name.eq("User1"))
        .all()
        .await
        .expect_err("query().all() should fail against an unreachable server");
    assert_connection("query().all()", &error);

    let error = Unreachable::query()
        .filter(|user| user.name.eq("User1"))
        .count()
        .await
        .expect_err("query().count() should fail against an unreachable server");
    assert_connection("query().count()", &error);

    let error = Unreachable::query()
        .filter(|user| user.active.eq(false))
        .delete_one()
        .await
        .expect_err("query().delete_one() should fail against an unreachable server");
    assert_connection("query().delete_one()", &error);

    let error = Unreachable::query()
        .filter(|user| user.active.eq(false))
        .delete_all()
        .await
        .expect_err("query().delete_all() should fail against an unreachable server");
    assert_connection("query().delete_all()", &error);

    let error = Unreachable::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.active.set(true))
        .await
        .expect_err("query().update_one() should fail against an unreachable server");
    assert_connection("query().update_one()", &error);

    let error = Unreachable::query()
        .filter(|user| user.name.eq("User1"))
        .update_all(|user| user.active.set(true))
        .await
        .expect_err("query().update_all() should fail against an unreachable server");
    assert_connection("query().update_all()", &error);

    Ok(())
}
