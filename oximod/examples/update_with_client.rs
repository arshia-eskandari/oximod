//! Update example for oximod using an explicit OxiClient
//!
//! Run with: `cargo run --example update_with_client`
//!
//! This demonstrates how to:
//! - Create an `OxiClient` instance with a MongoDB URI
//! - Obtain a `mongodb::Client` from it
//! - Clear a collection with `clear_with_client`
//! - Insert a document with `save_with_client`
//! - Update documents using `update_with_client` and `update_by_id_with_client`

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
    let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
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
    User::clear_with_client(client).await?;

    // Insert a user using the fluent builder + save_with_client
    let user = User::new().name("User1").age(45).active(false);

    let id = user.save_with_client(client).await?;
    println!("📝 Inserted user with _id: {}", id);

    // Generic update with explicit client:
    // Set active = true for all users over 40
    let result = User::update_with_client(
        doc! { "age": { "$gt": 40 } },
        doc! { "$set": { "active": true } },
        client,
    )
    .await?;
    println!("🔁 Updated {} document(s)", result.modified_count);

    // Update a single document by ID with explicit client
    let result =
        User::update_by_id_with_client(id, doc! { "$set": { "name": "User1 Updated" } }, client)
            .await?;
    println!(
        "🆔 Updated {} document(s) by ID using client",
        result.modified_count
    );

    Ok(())
}
