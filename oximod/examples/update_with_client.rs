//! Use OxiMod without initializing a global MongoDB client.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example update_with_client
//! ```
//!
//! This example demonstrates how to:
//!
//! - create an instance-level `OxiClient`;
//! - obtain its underlying `mongodb::Client`;
//! - use explicit-client model methods ending in `_from`;
//! - save, find, update, count, check, and delete documents;
//! - access a typed MongoDB collection with `get_collection_from()`;
//! - operate successfully without initializing OxiMod's global client.
//!
//! Explicit-client methods are useful for dependency injection, integration
//! tests, and applications that manage more than one MongoDB connection.
//!
//! Sessionless typed queries started with `Model::query()` currently use
//! OxiMod's global client; the `_with_session` terminals run through the
//! session's own client instead. For arbitrary queries in an explicit-client
//! workflow, use the typed collection returned by `get_collection_from()`.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

// A background job stored as an independent MongoDB document.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("explicit_client_jobs")]
struct BackgroundJob {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(non_empty)]
    reference: String,

    #[validate(non_empty)]
    queue: String,

    #[default("queued".to_string())]
    status: String,

    #[validate(non_negative)]
    #[default(0)]
    attempts: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    worker: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    // `OxiClient::new()` creates an instance-level client. It does not install
    // that client into OxiMod's process-wide global slot.
    let oxi_client = OxiClient::new(mongodb_uri).await?;

    // A client created successfully with `OxiClient::new()` always contains
    // its MongoDB driver client.
    let client = oxi_client
        .client()
        .expect("a newly created OxiClient should contain a MongoDB client");

    println!("Global client initialized: {}", OxiClient::global().is_ok());

    // Every operation in this example receives `client` explicitly.
    //
    // This collection belongs only to the example, so clearing it keeps
    // repeated runs deterministic.
    BackgroundJob::clear_from(client).await?;

    let job1 = BackgroundJob::new().reference("Job1").queue("email");

    let job2 = BackgroundJob::new().reference("Job2").queue("reports");

    let job1_id = job1.save_from(client).await?;
    let job2_id = job2.save_from(client).await?;

    println!("Saved Job1 with _id: {job1_id}");
    println!("Saved Job2 with _id: {job2_id}");

    let stored_jobs = BackgroundJob::count_from(doc! {}, client).await?;

    println!("Stored jobs: {stored_jobs}");

    let queued_email_job_exists = BackgroundJob::exists_from(
        doc! {
            "queue": "email",
            "status": "queued",
        },
        client,
    )
    .await?;

    println!("Queued email job exists: {queued_email_job_exists}");

    let saved_job1 = BackgroundJob::find_by_id_from(job1_id, client)
        .await?
        .expect("Job1 should exist after being saved");

    println!(
        "Found by ID: {} | queue: {} | status: {}",
        saved_job1.reference, saved_job1.queue, saved_job1.status,
    );

    // The explicit-client ID helper accepts an ordinary MongoDB update
    // document and returns the driver's UpdateResult.
    let update_result = BackgroundJob::update_by_id_from(
        job1_id,
        doc! {
            "$set": {
                "status": "running",
                "worker": "Worker1",
            },
            "$inc": {
                "attempts": 1,
            },
        },
        client,
    )
    .await?;

    println!(
        "Update result: {} matched, {} modified",
        update_result.matched_count, update_result.modified_count,
    );

    let updated_job1 = BackgroundJob::find_by_id_from(job1_id, client)
        .await?
        .expect("Job1 should still exist after its update");

    println!(
        "Updated Job1: status {}, attempts {}, worker {}",
        updated_job1.status,
        updated_job1.attempts,
        updated_job1.worker.as_deref().unwrap_or("unassigned"),
    );

    // Explicit-client workflows still retain direct access to MongoDB's typed
    // collection API. The result is deserialized into `BackgroundJob`.
    let collection = BackgroundJob::get_collection_from(client)?;

    let reports_job = collection
        .find_one(doc! {
            "queue": "reports",
        })
        .await?
        .expect("Job2 should be present in the reports queue");

    println!(
        "Typed collection lookup: {} in the {} queue",
        reports_job.reference, reports_job.queue,
    );

    let delete_result = BackgroundJob::delete_by_id_from(job2_id, client).await?;

    println!(
        "Deleted Job2: {} document deleted",
        delete_result.deleted_count
    );

    let remaining_jobs = BackgroundJob::count_from(doc! {}, client).await?;

    println!("Remaining jobs: {remaining_jobs}");

    Ok(())
}
