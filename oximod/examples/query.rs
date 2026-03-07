//! Query example for oximod
//!
//! Run with: `cargo run --example query`
//!
//! This demonstrates how to:
//! - Insert documents
//! - Query with `find`
//! - Query with `find_one`
//! - Check if a document exists

use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    OxiClient::init_global(mongodb_uri).await?;

    #[derive(Debug, Serialize, Deserialize, Model)]
    #[db("query_example_db")]
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

    // Insert multiple users using builder API
    let users = vec![
        User::new().name("Alice").age(30).active(true),
        User::new().name("Bob").age(40).active(false),
        User::new().name("Charlie").age(25).active(true),
    ];

    for user in &users {
        let id = user.save().await?;
        println!("📝 Inserted user {} with _id: {}", user.name, id);
    }

    let collection = User::get_collection()?;

    // Query all active users
    let cursor = collection.find(doc! { "active": true }).await?;
    let active_users: Vec<User> = cursor.try_collect().await?;

    println!("\n✅ Active users:");
    for user in active_users {
        println!("- {} (age: {})", user.name, user.age);
    }

    // Find one user named Bob
    if let Some(bob) = collection.find_one(doc! { "name": "Bob" }).await? {
        println!("\n🔍 Found Bob (age: {})", bob.age);
    }

    // Check if any inactive user exists
    let exists = collection
        .find_one(doc! { "active": false })
        .await?
        .is_some();

    println!("\n❓ Is there any inactive user? {}", exists);

    Ok(())
}
