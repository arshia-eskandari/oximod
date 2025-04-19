//! Query example for oximod
//!
//! Run with: `cargo run --example query`
//!
//! This demonstrates how to:
//! - Insert documents
//! - Query with `find`
//! - Query with `find_one`
//! - Check if a document exists

use oximod::{set_global_client, Model};
use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    set_global_client(mongodb_uri).await?;

    #[derive(Debug, Serialize, Deserialize, Model)]
    #[db("query_example_db")]
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

    // Insert multiple users
    let users = vec![
        User { _id: None, name: "Alice".into(), age: 30, active: true },
        User { _id: None, name: "Bob".into(), age: 40, active: false },
        User { _id: None, name: "Charlie".into(), age: 25, active: true },
    ];

    for user in &users {
        let id = user.save().await?;
        println!("📝 Inserted user {} with _id: {}", user.name, id);
    }

    // Query all active users
    let active_users = User::find(doc! { "active": true }).await?;
    println!("\n✅ Active users:");
    for user in active_users {
        println!("- {} (age: {})", user.name, user.age);
    }

    // Find one user named Bob
    if let Some(bob) = User::find_one(doc! { "name": "Bob" }).await? {
        println!("\n🔍 Found Bob (age: {})", bob.age);
    }

    // Check if any inactive user exists
    let exists = User::exists(doc! { "active": false }).await?;
    println!("\n❓ Is there any inactive user? {}", exists);

    Ok(())
}
