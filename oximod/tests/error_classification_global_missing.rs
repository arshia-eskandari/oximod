//! Failure-class contract: operations without an initialized global client
//! (SR-2).
//!
//! This binary requires the process-wide global client to remain completely
//! uninitialized, a state that is incompatible with every test that
//! initializes it. It therefore lives in its own integration-test binary,
//! which keeps it deterministic under ordinary `cargo test` (one process per
//! binary) as well as under nextest's process-per-test model. No test added
//! to this binary may initialize the global client.
//!
//! This binary does not require `MONGODB_URI` or a running deployment.

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiModError, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

// Run test: cargo nextest run global_client_missing_remains_distinct
#[tokio::test]
async fn global_client_missing_remains_distinct() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("error_class_global_missing")]
    pub struct NoGlobal {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
    }

    let error = NoGlobal::default()
        .name("User1")
        .save()
        .await
        .expect_err("save should fail without a global client");
    assert!(
        matches!(error, OxiModError::GlobalClientMissing { .. }),
        "expected save() without a global client to remain \
         GlobalClientMissing, got: {error:?}"
    );

    let error = NoGlobal::query()
        .filter(|user| user.name.eq("User1"))
        .first()
        .await
        .expect_err("query().first() should fail without a global client");
    assert!(
        matches!(error, OxiModError::GlobalClientMissing { .. }),
        "expected query().first() without a global client to remain \
         GlobalClientMissing, got: {error:?}"
    );

    Ok(())
}
