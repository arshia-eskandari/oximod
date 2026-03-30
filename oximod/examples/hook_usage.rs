//! Hook usage example for the oximod crate
//!
//! Run with: `cargo run --example hook_usage`
//!
//! This demonstrates how to:
//! - Implement the `Hooks` trait on a model
//! - Use immutable save hooks with `save()`
//! - Use mutable save hooks with `save_mut()`
//! - Use update and delete hooks

use mongodb::bson::{DateTime, doc, oid::ObjectId};
use oximod::{Hooks, Model, OxiClient};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("hook_example_db")]
#[collection("logs")]
struct Log {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    message: String,
    timestamp: i64,
    normalized: bool,
}

#[async_trait::async_trait]
impl Hooks for Log {
    async fn pre_save(&self) -> Result<(), oximod::OxiModError> {
        println!(
            "🔎 pre_save: validating immutable save for '{}'",
            self.message
        );
        Ok(())
    }

    async fn post_save(&self) -> Result<(), oximod::OxiModError> {
        println!(
            "✅ post_save: immutable save completed for '{}'",
            self.message
        );
        Ok(())
    }

    async fn pre_save_mut(&mut self) -> Result<(), oximod::OxiModError> {
        self.message = self.message.trim().to_string();
        self.normalized = true;

        println!("🛠️ pre_save_mut: normalized message to '{}'", self.message);

        Ok(())
    }

    async fn post_save_mut(&mut self) -> Result<(), oximod::OxiModError> {
        println!(
            "✅ post_save_mut: mutable save completed for '{}' (normalized = {})",
            self.message, self.normalized
        );
        Ok(())
    }

    async fn pre_update(
        id: ObjectId,
        update: &mongodb::bson::Document,
    ) -> Result<(), oximod::OxiModError> {
        println!("✏️ pre_update: updating document {} with {:?}", id, update);
        Ok(())
    }

    async fn post_update(
        id: ObjectId,
        update: &mongodb::bson::Document,
    ) -> Result<(), oximod::OxiModError> {
        println!("✅ post_update: updated document {} with {:?}", id, update);
        Ok(())
    }

    async fn pre_delete(id: ObjectId) -> Result<(), oximod::OxiModError> {
        println!("🗑️ pre_delete: deleting document {}", id);
        Ok(())
    }

    async fn post_delete(id: ObjectId) -> Result<(), oximod::OxiModError> {
        println!("✅ post_delete: deleted document {}", id);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    OxiClient::init_global(mongodb_uri).await?;

    Log::clear().await?;

    println!("📥 Inserting log entry with immutable save...");
    let immutable_log = Log::default()
        .message("System started")
        .timestamp(DateTime::now().timestamp_millis())
        .normalized(false);

    let immutable_id = immutable_log.save().await?;
    println!("✅ Saved immutable log with _id: {}", immutable_id);

    println!();
    println!("📥 Inserting log entry with mutable save...");
    let mut mutable_log = Log::default()
        .message("   Background worker started   ")
        .timestamp(DateTime::now().timestamp_millis())
        .normalized(false);

    let mutable_id = mutable_log.save_mut().await?;
    println!("✅ Saved mutable log with _id: {}", mutable_id);
    println!(
        "📋 In-memory state after save_mut -> message: '{}', normalized: {}",
        mutable_log.message, mutable_log.normalized
    );

    println!();
    println!("✏️ Updating mutable log...");
    Log::update_by_id(
        mutable_id,
        doc! {
            "$set": {
                "message": "Worker ready"
            }
        },
    )
    .await?;

    println!();
    println!("🗑️ Deleting immutable log...");
    Log::delete_by_id(immutable_id).await?;

    Ok(())
}
