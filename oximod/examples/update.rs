//! Update example for oximod
//!
//! Run with: `cargo run --example update`
//!
//! This demonstrates how to:
//! - Insert a document
//! - Update fields using MongoDB update operators

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    OxiClient::init_global(mongodb_uri).await?;

    #[derive(Debug, Serialize, Deserialize, Model)]
    #[db("update_example_db")]
    #[collection("users")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        #[default(false)]
        active: bool,
    }

    // Clean up previous runs
    User::clear().await?;

    // Insert a user
    let user = User::new().name("User1").age(45).active(false);

    let id = user.save().await?;
    println!("📝 Inserted user with _id: {}", id);

    let collection = User::get_collection()?;

    // Generic update: Set active = true for all users over 40
    let result = collection
        .update_many(
            doc! { "age": { "$gt": 40 } },
            doc! { "$set": { "active": true } },
        )
        .await?;

    println!("🔁 Updated {} document(s)", result.modified_count);

    // Update by ID
    let result = collection
        .update_one(
            doc! { "_id": id },
            doc! { "$set": { "name": "User1 Updated" } },
        )
        .await?;

    println!("🆔 Updated {} document(s) by ID", result.modified_count);

    Ok(())
}
