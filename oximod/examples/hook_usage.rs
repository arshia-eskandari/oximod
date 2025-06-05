//! Hook usage example for the oximod crate
//!
//! Run with: `cargo run --example hook_usage`
//!
//! This demonstrates how to:
//! - Use a pre-save hook using an `impl` block
//! - Log or modify state before a document is saved

use oximod::{set_global_client, Model};
use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("hook_example_db")]
#[collection("logs")]
struct Log {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    message: String,
    timestamp: i64,
}

// Pre-save hook implementation
impl Log {
    fn print_message(self) -> Self {
        println!("📋 Log message: {}", self.message);
        self
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    set_global_client(mongodb_uri).await?;

    // Clear any previous entries
    Log::clear().await?;

    println!("📥 Inserting log entry...");
    let log = Log::default()
        .message("System started".to_string())
        .timestamp(DateTime::now().timestamp_millis())
        .print_message()
        .save().await?;
    println!("✅ Saved log with _id: {}", log);

    Ok(())
}
