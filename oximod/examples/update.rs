//! Update example for oximod
//!
//! Run with: `cargo run --example update`
//!
//! This demonstrates how to:
//! - Insert a document
//! - Update fields using `update` and `update_by_id`

use oximod::{set_global_client, Model};
use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    set_global_client(mongodb_uri).await?;

    #[derive(Debug, Serialize, Deserialize, Model)]
    #[db("update_example_db")]
    #[collection("users")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    // Insert a user
    let user = User {
        _id: None,
        name: "User1".to_string(),
        age: 45,
        active: false,
    };

    // Clean up previous runs
    User::clear().await?;

    let id = user.save().await?;
    println!("📝 Inserted user with _id: {}", id);

    // Generic update: Set active = true for all users over 40
    let result = User::update(
        doc! { "age": { "$gt": 40 } },
        doc! { "$set": { "active": true } },
    )
    .await?;
    println!("🔁 Updated {} document(s)", result.modified_count);

    // Update by ID
    let result = User::update_by_id(
        id,
        doc! { "$set": { "name": "User1 Updated" } },
    )
    .await?;
    println!("🆔 Updated {} document(s) by ID", result.modified_count);

    Ok(())
}
