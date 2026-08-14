//! Batch inserts, typed updates, and typed deletions in one MongoDB
//! `bulkWrite` command.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example bulk_write
//! ```
//!
//! This example demonstrates a realistic queue-import workflow:
//!
//! - define a model with a unique dedupe key and validation;
//! - initialize declared indexes explicitly before bulk ingest;
//! - queue mixed operation kinds (insert, typed update, typed delete) in one
//!   `Model::bulk_write()` batch;
//! - execute unordered with verbose per-operation results;
//! - inspect a partial failure's original operation index.
//!
//! Bulk writes require **MongoDB Server 8.0+**; OxiMod never falls back to
//! per-item writes on older servers.
//!
//! Bulk writes are a separate execution surface: queued whole-model inserts
//! and replacements run `#[validate]` before any network communication, but
//! **lifecycle hooks do not run** for bulk operations, and declared indexes
//! are never established implicitly — hence the explicit `init_indexes_from`
//! below.
//!
//! A `BulkWrite<M>` batch targets one model and therefore one collection.
//! For bulk writes spanning multiple namespaces — or driver capabilities
//! outside the typed surface — use `mongodb::Client::bulk_write` directly.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiClient, Queryable};
use serde::{Deserialize, Serialize};

// A work-queue job guarded by a unique dedupe key, so re-importing the same
// feed cannot enqueue duplicates.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("bulk_write_jobs")]
struct Job {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(unique, name = "job_dedupe_key_idx")]
    #[validate(non_empty)]
    dedupe_key: String,

    #[validate(non_empty)]
    status: String,

    #[validate(non_negative)]
    attempts: i32,
}

fn job(dedupe_key: &str, status: &str) -> Job {
    Job::new().dedupe_key(dedupe_key).status(status).attempts(0)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    let owner = OxiClient::new(mongodb_uri).await?;
    let client = owner
        .client()
        .expect("OxiClient::new initializes its client");

    Job::clear_from(client).await?;

    // Establish the unique idempotency index before relying on it. Bulk
    // writes never create declared indexes implicitly.
    Job::init_indexes_from(client).await?;

    // Seed two jobs that the batch below will update and delete.
    job("import-0001", "stale").save_from(client).await?;
    job("import-0002", "expired").save_from(client).await?;

    // One mixed batch: three inserts, one typed update, one typed deletion —
    // submitted to MongoDB as one driver bulk-write action in exactly this
    // order, never as per-item OxiMod writes. Every queued whole-model
    // insert is validated before anything is sent.
    let result = Job::bulk_write()
        .insert_many([
            job("import-0003", "queued"),
            job("import-0004", "queued"),
            job("import-0005", "queued"),
        ])
        .update_one(
            Job::query().filter(|job| job.dedupe_key.eq("import-0001")),
            |job| job.status.set("requeued") & job.attempts.inc(1),
        )
        .delete_many(Job::query().filter(|job| job.status.eq("expired")))
        .ordered(false)
        .execute_verbose_from(client)
        .await?;

    println!("--- mixed batch (unordered, verbose) ---");
    println!("inserted: {}", result.summary.inserted_count);
    println!("matched:  {}", result.summary.matched_count);
    println!("modified: {}", result.summary.modified_count);
    println!("deleted:  {}", result.summary.deleted_count);

    // Verbose results are keyed by each operation's original queued index.
    for (index, insert) in &result.insert_results {
        println!("operation {index} inserted _id {:?}", insert.inserted_id);
    }

    // Re-importing part of the same feed demonstrates partial failure: two
    // duplicates collide with the unique index while the new job lands,
    // because unordered execution continues past failed operations.
    let error = Job::bulk_write()
        .insert_many([
            job("import-0003", "queued"), // duplicate -> fails at index 0
            job("import-0006", "queued"), // new -> succeeds at index 1
            job("import-0004", "queued"), // duplicate -> fails at index 2
        ])
        .ordered(false)
        .execute_from(client)
        .await
        .expect_err("the duplicate dedupe keys must fail their operations");

    println!("--- partial failure (unordered re-import) ---");

    // The driver's failure detail survives inside the OxiMod error: each
    // failed write maps back to its original queued operation index, and
    // the successful portion of the batch is preserved as a partial result.
    let failure = error
        .bulk_write_error()
        .expect("duplicate-key failures carry driver bulk-write detail");

    let mut failed_indexes: Vec<_> = failure.write_errors.keys().copied().collect();
    failed_indexes.sort_unstable();
    println!("failed operation indexes: {failed_indexes:?}");

    for index in failed_indexes {
        let write_error = &failure.write_errors[&index];
        println!(
            "operation {index} failed with server code {}: {}",
            write_error.code, write_error.message
        );
    }

    if let Some(partial) = &failure.partial_result {
        println!("partial result for the successful operations: {partial:?}");
    }

    let landed = Job::count_from(mongodb::bson::doc! {}, client).await?;
    println!("jobs stored after both batches: {landed}");

    Ok(())
}
