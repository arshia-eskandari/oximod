//! Basic usage example for the oximod crate
//!
//! Run with: `cargo run --example basic_usage`
//!
//! This demonstrates how to:
//! - Connect to MongoDB
//! - Define a model with the `Model` derive macro
//! - Save a document
//! - Count documents in a collection


use oximod::{set_global_client, Model};
use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load MongoDB URI from .env or environment
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in your .env file or environment");

    // Set up the global MongoDB client
    set_global_client(mongodb_uri).await?;

    // Define your model
    #[derive(Debug, Serialize, Deserialize, Model)]
    #[db("basic_usage_db")]
    #[collection("users")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    // Clean up previous runs
    User::clear().await?;

    // Create a new user
    let user = User {
        _id: None,
        name: "User1".to_string(),
        age: 28,
        active: true,
    };

    // Save the user and retrieve the inserted _id
    let id = user.save().await?;
    println!("✅ Saved user with _id: {}", id);

    // Count all users in the collection
    let count = User::count(doc! {}).await?;
    println!("📊 There are {} user(s) in the collection.", count);

    Ok(())
}
