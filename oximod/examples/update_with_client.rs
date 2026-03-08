//! Update example for oximod using an explicit OxiClient
//!
//! Run with: `cargo run --example update_with_client`
//!
//! This demonstrates how to:
//! - Create an `OxiClient` instance with a MongoDB URI
//! - Obtain a `mongodb::Client` from it
//! - Clear a collection with `clear_from`
//! - Insert a document with `save_from`
//! - Update documents using the raw MongoDB collection API

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables (including MONGODB_URI)
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("Missing MONGODB_URI env var. Please set it before running this example.");

    // Create a scoped OxiClient instance and grab the underlying mongodb::Client
    let oxiclient = OxiClient::new(mongodb_uri).await?;
    let client = oxiclient.client().expect("OxiClient has no inner client");

    #[derive(Debug, Serialize, Deserialize, Model)]
    #[db("update_with_client_example_db")]
    #[collection("users")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        #[default(false)]
        active: bool,
    }

    // Clean up previous runs using the explicit client
    User::clear_from(client).await?;

    // Insert a user using the fluent builder + save_from
    let user = User::new().name("User1").age(45).active(false);

    let id = user.save_from(client).await?;
    println!("📝 Inserted user with _id: {}", id);

    let collection = User::get_collection_from(client)?;

    // Generic update with explicit client:
    // Set active = true for all users over 40
    let result = collection
        .update_many(
            doc! { "age": { "$gt": 40 } },
            doc! { "$set": { "active": true } },
        )
        .await?;
    println!("🔁 Updated {} document(s)", result.modified_count);

    // Update a single document by ID with explicit client
    let result = collection
        .update_one(
            doc! { "_id": id },
            doc! { "$set": { "name": "User1 Updated" } },
        )
        .await?;
    println!(
        "🆔 Updated {} document(s) by ID using client",
        result.modified_count
    );

    Ok(())
}
