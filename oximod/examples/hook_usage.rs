//! Add lifecycle hooks to collection-backed models.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example hook_usage
//! ```
//!
//! This example demonstrates how to:
//!
//! - enable generated hook calls with `#[hooks]`;
//! - reject an operation from a pre-hook;
//! - observe immutable saves with `pre_save()` and `post_save()`;
//! - normalize a model in place with `pre_save_mut()`;
//! - distinguish pre-save and post-save mutation;
//! - observe find, update, and delete operations performed by `_id`;
//! - understand which operations do not run lifecycle hooks.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Hooks, Model, OxiClient, OxiModError, Queryable};
use serde::{Deserialize, Serialize};

// A support note stored as an independent MongoDB document.
//
// Implementing `Hooks` alone is not enough. The `#[hooks]` attribute tells the
// derive macro to invoke the implementation from generated persistence methods.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("hook_support_notes")]
#[hooks]
struct SupportNote {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(min_length = 5)]
    message: String,

    #[default(false)]
    normalized: bool,

    #[default(false)]
    post_processed: bool,
}

#[async_trait::async_trait]
impl Hooks for SupportNote {
    /// Runs before `save()` and can inspect, but not mutate, the model.
    async fn pre_save(&self) -> Result<(), OxiModError> {
        println!("pre_save: checking immutable note '{}'", self.message);

        if self.message.contains("[blocked]") {
            return Err(OxiModError::custom(
                "notes containing `[blocked]` cannot be saved",
            ));
        }

        Ok(())
    }

    /// Runs after `save()` has successfully inserted the document.
    async fn post_save(&self) -> Result<(), OxiModError> {
        println!("post_save: immutable note '{}' was inserted", self.message);

        Ok(())
    }

    /// Runs before validation and insertion in `save_mut()`.
    ///
    /// Mutations made here are both visible to validation and persisted.
    async fn pre_save_mut(&mut self) -> Result<(), OxiModError> {
        println!("pre_save_mut: received '{}'", self.message);

        self.message = self
            .message
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        self.normalized = true;

        println!("pre_save_mut: normalized to '{}'", self.message);

        Ok(())
    }

    /// Runs after insertion in `save_mut()`.
    ///
    /// Mutations made here affect the Rust value but are not automatically
    /// written back to the document that was already inserted.
    async fn post_save_mut(&mut self) -> Result<(), OxiModError> {
        self.post_processed = true;

        println!("post_save_mut: marked the in-memory value as post-processed");

        Ok(())
    }

    async fn pre_find(id: ObjectId) -> Result<(), OxiModError> {
        println!("pre_find: looking up note {id}");
        Ok(())
    }

    async fn post_find(result: &Option<Self>) -> Result<(), OxiModError> {
        match result {
            Some(note) => {
                println!("post_find: found '{}'", note.message);
            }
            None => {
                println!("post_find: no note was found");
            }
        }

        Ok(())
    }

    async fn pre_update(id: ObjectId, update: &mongodb::bson::Document) -> Result<(), OxiModError> {
        println!("pre_update: applying {update:?} to {id}");
        Ok(())
    }

    async fn post_update(
        id: ObjectId,
        _update: &mongodb::bson::Document,
    ) -> Result<(), OxiModError> {
        println!("post_update: update completed for {id}");
        Ok(())
    }

    async fn pre_delete(id: ObjectId) -> Result<(), OxiModError> {
        println!("pre_delete: deleting note {id}");
        Ok(())
    }

    async fn post_delete(id: ObjectId) -> Result<(), OxiModError> {
        println!("post_delete: deletion completed for {id}");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // `clear()` does not run model hooks. Hooks wrap only the corresponding
    // save and `_id` helper operations.
    SupportNote::clear().await?;

    println!("Immutable save");
    println!("--------------");

    let immutable_note = SupportNote::new().message("System started successfully");

    let immutable_id = immutable_note.save().await?;

    println!("saved immutable note as {immutable_id}");

    println!();
    println!("Rejected immutable save");
    println!("-----------------------");

    let blocked_note = SupportNote::new().message("[blocked] internal note");

    match blocked_note.save().await {
        Ok(id) => {
            println!("unexpectedly saved blocked note as {id}");
        }
        Err(error) => {
            println!("save was stopped by pre_save: {error}");
        }
    }

    println!();
    println!("Mutable save");
    println!("------------");

    let mut mutable_note = SupportNote::new().message("  Customer   requested   a callback  ");

    let mutable_id = mutable_note.save_mut().await?;

    println!("saved mutable note as {mutable_id}");
    println!(
        "in-memory state: message='{}', normalized={}, \
         post_processed={}",
        mutable_note.message, mutable_note.normalized, mutable_note.post_processed,
    );

    let stored_after_save = SupportNote::find_by_id(mutable_id)
        .await?
        .expect("mutable note should exist after saving");

    println!(
        "stored state: normalized={}, post_processed={}",
        stored_after_save.normalized, stored_after_save.post_processed,
    );

    // `post_save_mut()` changed only the in-memory value, so the stored
    // `post_processed` field remains false.
    assert!(mutable_note.post_processed);
    assert!(!stored_after_save.post_processed);

    println!();
    println!("Update by ID");
    println!("------------");

    let update_result = SupportNote::update_by_id(
        mutable_id,
        doc! {
            "$set": {
                "message": "Customer callback completed",
            },
        },
    )
    .await?;

    println!(
        "update result: {} matched, {} modified",
        update_result.matched_count, update_result.modified_count,
    );

    println!();
    println!("Typed query");
    println!("-----------");

    // Typed-query execution does not invoke lifecycle hooks. It has its own
    // direct query path and is intentionally independent from `find_by_id()`.
    let queried_note = SupportNote::query()
        .filter(|note| note.normalized.eq(true))
        .first()
        .await?
        .expect("the normalized note should be queryable");

    println!("typed query found: '{}'", queried_note.message);

    println!();
    println!("Delete by ID");
    println!("------------");

    let delete_result = SupportNote::delete_by_id(immutable_id).await?;

    println!(
        "delete result: {} document deleted",
        delete_result.deleted_count
    );

    let stored_count = SupportNote::query().count().await?;

    println!();
    println!("Stored notes after workflow: {stored_count}");

    Ok(())
}
