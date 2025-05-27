//! By ID example for oximod
//!
//! Run with: `cargo run --example by_id`
//!
//! This demonstrates how to:
//! - Insert a document
//! - Find a document by its `_id`
//! - Update a document by its `_id`
//! - Delete a document by its `_id`

use oximod::{ set_global_client, Model };
use mongodb::bson::{ doc, oid::ObjectId };
use serde::{ Deserialize, Serialize };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load MongoDB URI from environment or .env
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    set_global_client(mongodb_uri).await?;

    #[derive(Debug, Serialize, Deserialize, Model)]
    #[db("by_id_example_db")]
    #[collection("users")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        #[default(true)]
        active: bool,
    }

    // Clean up previous runs
    User::clear().await?;

    // Insert one user using the builder API
    let user = User::new().name("User1".to_string()).age(35); // `active` defaults to true

    let id = user.save().await?;
    println!("✅ Inserted user with _id: {}", id);

    // Find by _id
    if let Some(found) = User::find_by_id(id).await? {
        println!("🔍 Found user: {} (age {})", found.name, found.age);
    }

    // Update by _id
    let update_result = User::update_by_id(id, doc! { "$set": { "active": false } }).await?;
    println!("♻️  Modified {} document(s)", update_result.modified_count);

    // Delete by _id
    let delete_result = User::delete_by_id(id).await?;
    println!("🗑️  Deleted {} document(s)", delete_result.deleted_count);

    Ok(())
}
