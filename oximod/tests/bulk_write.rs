//! Integration tests for typed model-scoped bulk writes.
//!
//! These tests cover the audit-driven bulk-write workflows: single
//! bulk-execution enqueue (A7), the ordered/unordered partial-failure matrix with driver
//! per-operation failure indexes (A6), all-before-network validation, mixed
//! operation kinds with verbose results, query-modifier preflight, session
//! transactions, the lifecycle-hook boundary, and error-classification
//! precedence.
//!
//! Most tests use an explicit `mongodb::Client` created from `MONGODB_URI`.
//! Bulk writes require MongoDB Server 8.0+, which the replica-set test
//! environment provides.

use std::error::Error as StdError;
use std::sync::atomic::{AtomicUsize, Ordering};

use mongodb::{
    Client,
    bson::{doc, oid::ObjectId},
    error::{ErrorKind, PartialBulkWriteResult},
    options::InsertOneModel,
};
use oximod::{BulkWriteOperation, Hooks, Model, OxiModError, QueryError, QueryModifier, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;

async fn client() -> Result<Client, Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
    Ok(Client::with_uri_str(uri).await?)
}

/// Returns the preserved `mongodb::error::Error` behind an `OxiModError`.
fn driver_source(error: &OxiModError) -> Option<&mongodb::error::Error> {
    error
        .source()
        .and_then(|source| source.downcast_ref::<mongodb::error::Error>())
}

/// Returns the inserted count recorded in a partial bulk-write result.
fn partial_inserted_count(partial: &PartialBulkWriteResult) -> i64 {
    match partial {
        PartialBulkWriteResult::Summary(summary) => summary.inserted_count,
        PartialBulkWriteResult::Verbose(verbose) => verbose.summary.inserted_count,
    }
}

// --- D4: mixed operation kinds with verbose results ------------------------

// Run test: cargo nextest run mixed_operations_verbose_results_match_queued_indexes
#[tokio::test]
async fn mixed_operations_verbose_results_match_queued_indexes() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_mixed_operations")]
    pub struct Job {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        dedupe_key: String,
        #[serde(rename = "statusLabel")]
        status: String,
        attempts: i32,
    }

    let client = client().await?;
    Job::clear_from(&client).await?;

    for (key, status) in [
        ("job-a", "queued"),
        ("job-b", "queued"),
        ("job-c", "stale"),
        ("job-d", "stale"),
        ("job-e", "expired"),
        ("job-f", "expired"),
    ] {
        Job::new()
            .dedupe_key(key)
            .status(status)
            .attempts(0)
            .save_from(&client)
            .await?;
    }

    // Queue index:      0       1           2            3            4           5
    // Operation kind:   insert  update_one  update_many  replace_one  delete_one  delete_many
    let result = Job::bulk_write()
        .insert(Job::new().dedupe_key("job-g").status("queued").attempts(0))
        .update_one(
            Job::query().filter(|job| job.dedupe_key.eq("job-a")),
            |job| job.status.set("ready") & job.attempts.inc(1),
        )
        .update_many(Job::query().filter(|job| job.status.eq("stale")), |job| {
            job.status.set("archived")
        })
        .replace_one(
            Job::query().filter(|job| job.dedupe_key.eq("job-b")),
            Job::new()
                .dedupe_key("job-b")
                .status("replaced")
                .attempts(9),
        )
        .delete_one(Job::query().filter(|job| job.dedupe_key.eq("job-e")))
        .delete_many(Job::query().filter(|job| job.status.eq("expired")))
        .execute_verbose_from(&client)
        .await?;

    // Summary counts across the whole batch.
    assert_eq!(result.summary.inserted_count, 1);
    assert_eq!(
        result.summary.matched_count, 4,
        "update_one + update_many(2) + replace_one"
    );
    assert_eq!(result.summary.modified_count, 4);
    assert_eq!(result.summary.upserted_count, 0);
    assert_eq!(
        result.summary.deleted_count, 2,
        "delete_one + delete_many(1)"
    );

    // Verbose per-operation results are keyed by original queued index.
    assert_eq!(
        result.insert_results.keys().copied().collect::<Vec<_>>(),
        vec![0],
    );
    let mut update_indexes: Vec<usize> = result.update_results.keys().copied().collect();
    update_indexes.sort_unstable();
    assert_eq!(update_indexes, vec![1, 2, 3]);
    let mut delete_indexes: Vec<usize> = result.delete_results.keys().copied().collect();
    delete_indexes.sort_unstable();
    assert_eq!(delete_indexes, vec![4, 5]);

    assert_eq!(result.update_results[&2].matched_count, 2);
    assert_eq!(result.delete_results[&4].deleted_count, 1);
    assert_eq!(result.delete_results[&5].deleted_count, 1);

    // Final collection state agrees with an independently computed oracle,
    // read through the raw driver collection.
    let raw = Job::get_document_collection_from(&client)?;
    assert_eq!(raw.count_documents(doc! {}).await?, 5);
    assert_eq!(
        raw.count_documents(doc! { "dedupe_key": "job-g" }).await?,
        1,
        "queued insert should be stored"
    );
    assert_eq!(
        raw.count_documents(doc! { "dedupe_key": "job-a", "statusLabel": "ready", "attempts": 1 })
            .await?,
        1,
        "update_one should target the Serde-renamed status field"
    );
    assert_eq!(
        raw.count_documents(doc! { "statusLabel": "archived" })
            .await?,
        2,
        "update_many should archive both stale jobs"
    );
    assert_eq!(
        raw.count_documents(
            doc! { "dedupe_key": "job-b", "statusLabel": "replaced", "attempts": 9 }
        )
        .await?,
        1,
        "replace_one should store the serialized replacement"
    );
    assert_eq!(
        raw.count_documents(doc! { "statusLabel": "expired" })
            .await?,
        0,
        "both expired jobs should be deleted"
    );

    Ok(())
}

// --- D1: A7-style 300-document enqueue --------------------------------------

// Run test: cargo nextest run a7_enqueue_300_documents_in_one_bulk_write
#[tokio::test]
async fn a7_enqueue_300_documents_in_one_bulk_write() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_a7_enqueue")]
    pub struct Job {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        #[index(unique)]
        dedupe_key: String,
        status: String,
    }

    let client = client().await?;
    Job::clear_from(&client).await?;

    // Establish the unique idempotency index explicitly before the batch.
    Job::init_indexes_from(&client).await?;

    let jobs = (0..300).map(|index| {
        Job::new()
            .dedupe_key(format!("enqueue-{index:03}"))
            .status("queued")
    });

    let result = Job::bulk_write()
        .insert_many(jobs)
        .execute_from(&client)
        .await?;

    assert_eq!(result.inserted_count, 300);

    // Independent raw-driver oracle: exactly 300 documents, all intended
    // dedupe keys present, no duplicates.
    let raw = Job::get_document_collection_from(&client)?;
    assert_eq!(raw.count_documents(doc! {}).await?, 300);

    let stored_keys = raw
        .distinct("dedupe_key", doc! {})
        .await?
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<std::collections::BTreeSet<_>>();
    let expected_keys = (0..300)
        .map(|index| format!("enqueue-{index:03}"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(stored_keys, expected_keys);

    Ok(())
}

// --- D2: A6-style 500-item partial-failure matrix ---------------------------

// Recreates the audit's core partial-failure semantics: 500 queued inserts
// with a duplicate dedupe key planted at operation index 249 (duplicating
// index 0), executed ordered and unordered through OxiMod and through a raw
// driver oracle on a fresh collection per arm.
//
// Run test: cargo nextest run a6_partial_failure_matrix_matches_raw_driver_oracle
#[tokio::test]
async fn a6_partial_failure_matrix_matches_raw_driver_oracle() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_a6_matrix")]
    pub struct Item {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        #[index(unique)]
        dedupe_key: String,
        payload: i32,
    }

    const TOTAL: usize = 500;
    const DUPLICATE_INDEX: usize = 249;

    fn key_for(index: usize) -> String {
        if index == DUPLICATE_INDEX {
            // The duplicate collides with the key queued at index 0.
            key_for(0)
        } else {
            format!("item-{index:03}")
        }
    }

    let client = client().await?;
    Item::init_indexes_from(&client).await?;

    /// Runs one OxiMod arm on a freshly cleared collection and returns the
    /// error and the landed document count.
    async fn oximod_arm(
        client: &Client,
        ordered: bool,
    ) -> Result<(OxiModError, u64), Box<dyn std::error::Error>> {
        Item::clear_from(client).await?;

        let items =
            (0..TOTAL).map(|index| Item::new().dedupe_key(key_for(index)).payload(index as i32));

        let error = Item::bulk_write()
            .insert_many(items)
            .ordered(ordered)
            .execute_from(client)
            .await
            .expect_err("the planted duplicate must fail the batch");

        let landed = Item::get_document_collection_from(client)?
            .count_documents(doc! {})
            .await?;

        Ok((error, landed))
    }

    /// Runs one raw-driver oracle arm on a freshly cleared collection and
    /// returns the driver bulk-write failure and the landed document count.
    async fn raw_arm(
        client: &Client,
        ordered: bool,
    ) -> Result<(mongodb::error::Error, u64), Box<dyn std::error::Error>> {
        Item::clear_from(client).await?;

        let collection = Item::get_collection_from(client)?;
        let models: Vec<InsertOneModel> = (0..TOTAL)
            .map(|index| {
                collection
                    .insert_one_model(Item::new().dedupe_key(key_for(index)).payload(index as i32))
            })
            .collect::<Result<_, _>>()?;

        let error = client
            .bulk_write(models)
            .ordered(ordered)
            .await
            .expect_err("the planted duplicate must fail the raw batch");

        let landed = Item::get_document_collection_from(client)?
            .count_documents(doc! {})
            .await?;

        Ok((error, landed))
    }

    /// Asserts the OxiMod arm's error contract and returns its partial
    /// inserted count.
    fn assert_oximod_error(error: &OxiModError, arm: &str) -> i64 {
        assert!(
            matches!(error, OxiModError::BulkWrite { .. }),
            "{arm}: expected OxiModError::BulkWrite, got: {error:?}"
        );

        // The original driver error is preserved through source() and still
        // exposes ErrorKind::BulkWrite.
        let source = driver_source(error)
            .unwrap_or_else(|| panic!("{arm}: driver error should be preserved as source"));
        assert!(
            matches!(&*source.kind, ErrorKind::BulkWrite(_)),
            "{arm}: expected ErrorKind::BulkWrite in the source, got: {source:?}"
        );

        // The convenience accessor reaches the same failure detail.
        let failure = error
            .bulk_write_error()
            .unwrap_or_else(|| panic!("{arm}: bulk_write_error() should reach the failure"));

        // The duplicate remains discoverable at its original queued index.
        assert_eq!(
            failure.write_errors.keys().copied().collect::<Vec<_>>(),
            vec![DUPLICATE_INDEX],
            "{arm}: the duplicate must map back to its queued operation index"
        );
        assert_eq!(
            failure.write_errors[&DUPLICATE_INDEX].code, 11000,
            "{arm}: the duplicate-key server code must be preserved"
        );

        let partial = failure
            .partial_result
            .as_ref()
            .unwrap_or_else(|| panic!("{arm}: partial_result should be preserved"));

        partial_inserted_count(partial)
    }

    /// Extracts the raw oracle arm's partial inserted count.
    fn raw_partial_inserted(error: &mongodb::error::Error, arm: &str) -> i64 {
        let ErrorKind::BulkWrite(failure) = &*error.kind else {
            panic!("{arm}: expected ErrorKind::BulkWrite, got: {error:?}");
        };

        assert_eq!(
            failure.write_errors.keys().copied().collect::<Vec<_>>(),
            vec![DUPLICATE_INDEX],
        );

        let partial = failure
            .partial_result
            .as_ref()
            .unwrap_or_else(|| panic!("{arm}: partial_result should be present"));

        partial_inserted_count(partial)
    }

    // Ordered arms: the server stops at the duplicate, landing exactly the
    // operations queued before it.
    let (oximod_ordered_error, oximod_ordered_landed) = oximod_arm(&client, true).await?;
    let oximod_ordered_partial = assert_oximod_error(&oximod_ordered_error, "oximod ordered");

    let (raw_ordered_error, raw_ordered_landed) = raw_arm(&client, true).await?;
    let raw_ordered_partial = raw_partial_inserted(&raw_ordered_error, "raw ordered");

    assert_eq!(oximod_ordered_landed, DUPLICATE_INDEX as u64);
    assert_eq!(oximod_ordered_landed, raw_ordered_landed);
    assert_eq!(oximod_ordered_partial, DUPLICATE_INDEX as i64);
    assert_eq!(oximod_ordered_partial, raw_ordered_partial);

    // Unordered arms: the server continues after the duplicate and lands
    // every non-duplicate operation.
    let (oximod_unordered_error, oximod_unordered_landed) = oximod_arm(&client, false).await?;
    let oximod_unordered_partial = assert_oximod_error(&oximod_unordered_error, "oximod unordered");

    let (raw_unordered_error, raw_unordered_landed) = raw_arm(&client, false).await?;
    let raw_unordered_partial = raw_partial_inserted(&raw_unordered_error, "raw unordered");

    assert_eq!(oximod_unordered_landed, (TOTAL - 1) as u64);
    assert_eq!(oximod_unordered_landed, raw_unordered_landed);
    assert_eq!(oximod_unordered_partial, (TOTAL - 1) as i64);
    assert_eq!(oximod_unordered_partial, raw_unordered_partial);

    // No duplicate dedupe key survives in any arm's final state (the last
    // arm's collection state is still available to check).
    let raw = Item::get_document_collection_from(&client)?;
    let stored = raw
        .count_documents(doc! { "dedupe_key": key_for(0) })
        .await?;
    assert_eq!(stored, 1, "the unique index must prevent a duplicate key");

    Ok(())
}

// --- D3: validation is all-before-network -----------------------------------

// Run test: cargo nextest run invalid_queued_insert_prevents_all_writes
#[tokio::test]
async fn invalid_queued_insert_prevents_all_writes() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_validation_insert")]
    pub struct Contact {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        #[validate(min_length = 3)]
        name: String,
    }

    let client = client().await?;
    Contact::clear_from(&client).await?;

    let error = Contact::bulk_write()
        .insert(Contact::new().name("valid-one"))
        .insert(Contact::new().name("valid-two"))
        .insert(Contact::new().name("no"))
        .insert(Contact::new().name("valid-three"))
        .execute_from(&client)
        .await
        .expect_err("an invalid queued insert must fail the whole batch");

    assert!(
        matches!(error, OxiModError::Validation(_)),
        "expected OxiModError::Validation, got: {error:?}"
    );

    let landed = Contact::get_document_collection_from(&client)?
        .count_documents(doc! {})
        .await?;
    assert_eq!(landed, 0, "validation must prevent every queued write");

    Ok(())
}

// Run test: cargo nextest run invalid_queued_replacement_prevents_all_writes
#[tokio::test]
async fn invalid_queued_replacement_prevents_all_writes() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_validation_replace")]
    pub struct Contact {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        #[validate(min_length = 3)]
        name: String,
    }

    let client = client().await?;
    Contact::clear_from(&client).await?;
    Contact::new().name("existing").save_from(&client).await?;

    let error = Contact::bulk_write()
        .insert(Contact::new().name("valid-insert"))
        .replace_one(
            Contact::query().filter(|contact| contact.name.eq("existing")),
            Contact::new().name("no"),
        )
        .execute_from(&client)
        .await
        .expect_err("an invalid queued replacement must fail the whole batch");

    assert!(
        matches!(error, OxiModError::Validation(_)),
        "expected OxiModError::Validation, got: {error:?}"
    );

    let raw = Contact::get_document_collection_from(&client)?;
    assert_eq!(raw.count_documents(doc! {}).await?, 1, "no insert may land");
    assert_eq!(
        raw.count_documents(doc! { "name": "existing" }).await?,
        1,
        "the existing document must remain unreplaced"
    );

    Ok(())
}

// --- D5: query-modifier preflight -------------------------------------------

// Run test: cargo nextest run unsupported_modifier_fails_locally_and_sends_no_write
#[tokio::test]
async fn unsupported_modifier_fails_locally_and_sends_no_write() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_modifier_preflight")]
    pub struct Job {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        dedupe_key: String,
        attempts: i32,
    }

    let client = client().await?;
    Job::clear_from(&client).await?;

    // update_many cannot carry a sort.
    let error = Job::bulk_write()
        .insert(Job::new().dedupe_key("preflight-insert").attempts(0))
        .update_many(Job::query().sort_by(|job| job.attempts.asc()), |job| {
            job.attempts.inc(1)
        })
        .execute_from(&client)
        .await
        .expect_err("an unsupported modifier must fail the whole batch");

    assert_eq!(
        error.query_error(),
        Some(&QueryError::UnsupportedBulkWriteModifier {
            operation: BulkWriteOperation::UpdateMany,
            modifier: QueryModifier::Sort,
        }),
    );

    // delete_one cannot carry skip.
    let error = Job::bulk_write()
        .delete_one(Job::query().skip(2))
        .execute_from(&client)
        .await
        .expect_err("an unsupported modifier must fail the whole batch");

    assert_eq!(
        error.query_error(),
        Some(&QueryError::UnsupportedBulkWriteModifier {
            operation: BulkWriteOperation::DeleteOne,
            modifier: QueryModifier::Skip,
        }),
    );

    let landed = Job::get_document_collection_from(&client)?
        .count_documents(doc! {})
        .await?;
    assert_eq!(landed, 0, "a rejected batch must send zero writes");

    Ok(())
}

// Run test: cargo nextest run update_one_sort_is_transmitted
#[tokio::test]
async fn update_one_sort_is_transmitted() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_update_one_sort")]
    pub struct Job {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        dedupe_key: String,
        status: String,
        attempts: i32,
    }

    let client = client().await?;
    Job::clear_from(&client).await?;

    for (key, attempts) in [("low", 1), ("high", 5)] {
        Job::new()
            .dedupe_key(key)
            .status("queued")
            .attempts(attempts)
            .save_from(&client)
            .await?;
    }

    // Both documents match; the sort must select the one with the highest
    // attempts count.
    let result = Job::bulk_write()
        .update_one(
            Job::query()
                .filter(|job| job.status.eq("queued"))
                .sort_by(|job| job.attempts.desc()),
            |job| job.status.set("claimed"),
        )
        .execute_from(&client)
        .await?;

    assert_eq!(result.matched_count, 1);
    assert_eq!(result.modified_count, 1);

    let raw = Job::get_document_collection_from(&client)?;
    assert_eq!(
        raw.count_documents(doc! { "dedupe_key": "high", "status": "claimed" })
            .await?,
        1,
        "the typed sort must be transmitted to the server"
    );
    assert_eq!(
        raw.count_documents(doc! { "dedupe_key": "low", "status": "queued" })
            .await?,
        1,
        "the unsorted-first document must remain unchanged"
    );

    Ok(())
}

// --- D6: sessions and transactions ------------------------------------------

// Run test: cargo nextest run bulk_write_in_transaction_is_isolated_and_commits
#[tokio::test]
async fn bulk_write_in_transaction_is_isolated_and_commits() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_session_commit")]
    pub struct Job {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        #[index(unique)]
        dedupe_key: String,
        status: String,
    }

    let client = client().await?;
    Job::clear_from(&client).await?;
    Job::new()
        .dedupe_key("existing")
        .status("queued")
        .save_from(&client)
        .await?;

    // Initialize declared indexes before transactional work.
    Job::init_indexes_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let result = Job::bulk_write()
        .insert(Job::new().dedupe_key("tx-insert").status("queued"))
        .update_one(
            Job::query().filter(|job| job.dedupe_key.eq("existing")),
            |job| job.status.set("ready"),
        )
        .execute_with_session(&mut session)
        .await?;

    assert_eq!(result.inserted_count, 1);
    assert_eq!(result.modified_count, 1);

    // The same session observes the transaction-local changes...
    assert_eq!(Job::count_with_session(doc! {}, &mut session).await?, 2);
    let inside = Job::query()
        .filter(|job| job.status.eq("ready"))
        .count_with_session(&mut session)
        .await?;
    assert_eq!(inside, 1, "the session must see its own uncommitted update");

    // ...while an ordinary read outside the session does not.
    let raw = Job::get_document_collection_from(&client)?;
    assert_eq!(
        raw.count_documents(doc! { "dedupe_key": "tx-insert" })
            .await?,
        0,
        "a sessionless reader must not see the uncommitted insert"
    );
    assert_eq!(
        raw.count_documents(doc! { "status": "ready" }).await?,
        0,
        "a sessionless reader must not see the uncommitted update"
    );

    session.commit_transaction().await?;

    assert_eq!(
        raw.count_documents(doc! { "dedupe_key": "tx-insert" })
            .await?,
        1,
        "the committed insert must be visible"
    );
    assert_eq!(
        raw.count_documents(doc! { "status": "ready" }).await?,
        1,
        "the committed update must be visible"
    );

    Ok(())
}

// Run test: cargo nextest run bulk_write_in_transaction_rolls_back_on_abort
#[tokio::test]
async fn bulk_write_in_transaction_rolls_back_on_abort() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_session_abort")]
    pub struct Job {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        #[index(unique)]
        dedupe_key: String,
        status: String,
    }

    let client = client().await?;
    Job::clear_from(&client).await?;
    Job::new()
        .dedupe_key("keep")
        .status("queued")
        .save_from(&client)
        .await?;

    Job::init_indexes_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    Job::bulk_write()
        .insert(Job::new().dedupe_key("rollback-insert").status("queued"))
        .delete_many(Job::query().filter(|job| job.dedupe_key.eq("keep")))
        .execute_with_session(&mut session)
        .await?;

    // Verbose session execution participates in the same transaction.
    let verbose = Job::bulk_write()
        .insert(Job::new().dedupe_key("rollback-verbose").status("queued"))
        .execute_verbose_with_session(&mut session)
        .await?;
    assert_eq!(verbose.summary.inserted_count, 1);
    assert_eq!(
        verbose.insert_results.keys().copied().collect::<Vec<_>>(),
        vec![0]
    );

    assert_eq!(
        Job::count_with_session(doc! {}, &mut session).await?,
        2,
        "the session must see both uncommitted bulk writes"
    );

    session.abort_transaction().await?;

    let raw = Job::get_document_collection_from(&client)?;
    assert_eq!(raw.count_documents(doc! {}).await?, 1);
    assert_eq!(
        raw.count_documents(doc! { "dedupe_key": "keep" }).await?,
        1,
        "the aborted delete must be rolled back"
    );

    Ok(())
}

// --- D7: lifecycle hooks do not run -----------------------------------------

static BULK_PRE_SAVE_CALLS: AtomicUsize = AtomicUsize::new(0);
static BULK_POST_SAVE_CALLS: AtomicUsize = AtomicUsize::new(0);
static BULK_PRE_UPDATE_CALLS: AtomicUsize = AtomicUsize::new(0);
static BULK_POST_UPDATE_CALLS: AtomicUsize = AtomicUsize::new(0);
static BULK_PRE_DELETE_CALLS: AtomicUsize = AtomicUsize::new(0);
static BULK_POST_DELETE_CALLS: AtomicUsize = AtomicUsize::new(0);

// Run test: cargo nextest run bulk_write_does_not_invoke_lifecycle_hooks
#[tokio::test]
async fn bulk_write_does_not_invoke_lifecycle_hooks() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_hooks_boundary")]
    #[hooks]
    pub struct Job {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        dedupe_key: String,
        status: String,
    }

    #[async_trait::async_trait]
    impl Hooks for Job {
        async fn pre_save(&self) -> Result<(), OxiModError> {
            BULK_PRE_SAVE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn post_save(&self) -> Result<(), OxiModError> {
            BULK_POST_SAVE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn pre_update(
            _id: ObjectId,
            _update: &mongodb::bson::Document,
        ) -> Result<(), OxiModError> {
            BULK_PRE_UPDATE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn post_update(
            _id: ObjectId,
            _update: &mongodb::bson::Document,
        ) -> Result<(), OxiModError> {
            BULK_POST_UPDATE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn pre_delete(_id: ObjectId) -> Result<(), OxiModError> {
            BULK_PRE_DELETE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn post_delete(_id: ObjectId) -> Result<(), OxiModError> {
            BULK_POST_DELETE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    let client = client().await?;
    Job::clear_from(&client).await?;

    // The hook wiring itself must be live: an ordinary save fires save hooks.
    Job::new()
        .dedupe_key("hooked-save")
        .status("queued")
        .save_from(&client)
        .await?;
    assert_eq!(BULK_PRE_SAVE_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(BULK_POST_SAVE_CALLS.load(Ordering::SeqCst), 1);

    let result = Job::bulk_write()
        .insert(Job::new().dedupe_key("bulk-1").status("queued"))
        .insert(Job::new().dedupe_key("bulk-2").status("queued"))
        .update_one(
            Job::query().filter(|job| job.dedupe_key.eq("hooked-save")),
            |job| job.status.set("ready"),
        )
        .delete_one(Job::query().filter(|job| job.dedupe_key.eq("bulk-2")))
        .execute_from(&client)
        .await?;

    assert_eq!(result.inserted_count, 2);
    assert_eq!(result.modified_count, 1);
    assert_eq!(result.deleted_count, 1);

    // The bulk documents landed, but no additional lifecycle hook fired.
    assert_eq!(
        BULK_PRE_SAVE_CALLS.load(Ordering::SeqCst),
        1,
        "bulk insert must not run pre_save"
    );
    assert_eq!(
        BULK_POST_SAVE_CALLS.load(Ordering::SeqCst),
        1,
        "bulk insert must not run post_save"
    );
    assert_eq!(BULK_PRE_UPDATE_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(BULK_POST_UPDATE_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(BULK_PRE_DELETE_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(BULK_POST_DELETE_CALLS.load(Ordering::SeqCst), 0);

    Ok(())
}

// --- D8: error-classification precedence ------------------------------------

// Run test: cargo nextest run duplicate_key_bulk_failure_classifies_as_bulk_write
#[tokio::test]
async fn duplicate_key_bulk_failure_classifies_as_bulk_write() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_error_duplicate")]
    pub struct Item {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        #[index(unique)]
        dedupe_key: String,
    }

    let client = client().await?;
    Item::clear_from(&client).await?;
    Item::init_indexes_from(&client).await?;

    let error = Item::bulk_write()
        .insert(Item::new().dedupe_key("dup"))
        .insert(Item::new().dedupe_key("dup"))
        .execute_from(&client)
        .await
        .expect_err("the duplicate insert must fail the batch");

    assert!(
        matches!(error, OxiModError::BulkWrite { .. }),
        "expected OxiModError::BulkWrite, got: {error:?}"
    );

    let failure = error
        .bulk_write_error()
        .expect("the driver failure detail should be reachable");
    assert_eq!(
        failure.write_errors.keys().copied().collect::<Vec<_>>(),
        vec![1],
        "the failing insert must map back to queued index 1"
    );
    assert_eq!(failure.write_errors[&1].code, 11000);

    Ok(())
}

// Run test: cargo nextest run unreachable_server_bulk_write_classifies_as_connection
#[tokio::test]
async fn unreachable_server_bulk_write_classifies_as_connection() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_error_unreachable")]
    pub struct Item {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
    }

    let unreachable = Client::with_uri_str(
        "mongodb://127.0.0.1:1/?serverSelectionTimeoutMS=300&connectTimeoutMS=300",
    )
    .await?;

    let error = Item::bulk_write()
        .insert(Item::new().name("unsendable"))
        .execute_from(&unreachable)
        .await
        .expect_err("an unreachable server must fail the batch");

    assert!(
        matches!(error, OxiModError::Connection { .. }),
        "Connection must outrank the bulk-write domain, got: {error:?}"
    );
    assert!(
        driver_source(&error).is_some(),
        "the driver error must be preserved through source()"
    );

    Ok(())
}

// Run test: cargo nextest run unserializable_queued_insert_classifies_as_serialization
#[tokio::test]
async fn unserializable_queued_insert_classifies_as_serialization() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_error_serialization")]
    pub struct Metric {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        // u64::MAX cannot be represented as BSON, which makes serialization
        // fail while materializing the insert model.
        sample: u64,
    }

    let client = client().await?;
    Metric::clear_from(&client).await?;

    let error = Metric::bulk_write()
        .insert(Metric::new().name("ok").sample(1u64))
        .insert(Metric::new().name("overflow").sample(u64::MAX))
        .execute_from(&client)
        .await
        .expect_err("an unserializable queued insert must fail the batch");

    assert!(
        matches!(error, OxiModError::Serialization { .. }),
        "Serialization must outrank the bulk-write domain, got: {error:?}"
    );

    let landed = Metric::get_document_collection_from(&client)?
        .count_documents(doc! {})
        .await?;
    assert_eq!(landed, 0, "materialization failure must send zero writes");

    Ok(())
}

// Run test: cargo nextest run empty_batch_fails_deterministically_without_network
#[tokio::test]
async fn empty_batch_fails_deterministically_without_network() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_error_empty")]
    pub struct Item {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
    }

    // An unreachable client proves the driver rejects the empty batch
    // locally, before any server communication.
    let unreachable = Client::with_uri_str(
        "mongodb://127.0.0.1:1/?serverSelectionTimeoutMS=300&connectTimeoutMS=300",
    )
    .await?;

    let error = Item::bulk_write()
        .execute_from(&unreachable)
        .await
        .expect_err("an empty batch must be rejected");

    assert!(
        matches!(error, OxiModError::BulkWrite { .. }),
        "expected the driver's empty-batch rejection to classify as \
         BulkWrite, got: {error:?}"
    );

    let source = driver_source(&error).expect("the driver error must be preserved");
    assert!(
        matches!(&*source.kind, ErrorKind::InvalidArgument { .. }),
        "expected the driver's InvalidArgument rejection, got: {source:?}"
    );

    Ok(())
}

// --- Global-client execution -------------------------------------------------

// Run test: cargo nextest run bulk_write_executes_through_the_global_client
#[tokio::test]
async fn bulk_write_executes_through_the_global_client() -> TestResult {
    common::init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, Clone)]
    #[db("bulk_write_test")]
    #[collection("bulk_global_client")]
    pub struct Job {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        dedupe_key: String,
        status: String,
    }

    Job::clear().await?;

    let result = Job::bulk_write()
        .insert(Job::new().dedupe_key("global-1").status("queued"))
        .insert(Job::new().dedupe_key("global-2").status("queued"))
        .execute()
        .await?;
    assert_eq!(result.inserted_count, 2);

    let verbose = Job::bulk_write()
        .update_many(Job::query().filter(|job| job.status.eq("queued")), |job| {
            job.status.set("ready")
        })
        .execute_verbose()
        .await?;
    assert_eq!(verbose.summary.matched_count, 2);
    assert_eq!(verbose.summary.modified_count, 2);
    assert_eq!(
        verbose.update_results.keys().copied().collect::<Vec<_>>(),
        vec![0],
    );

    assert_eq!(Job::count(doc! { "status": "ready" }).await?, 2);

    Ok(())
}
