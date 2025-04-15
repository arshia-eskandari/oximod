//! Delete example for oximod
//!
//! Run with: `cargo run --example delete`
//!
//! This demonstrates how to:
//! - Insert users
//! - Delete multiple documents
//! - Delete a document by ID
//! - Delete one document with a filter

use oximod_core::feature::conn::client::set_global_client;
use oximod_core::feature::model::Model;
use oximod_macros::Model;
use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    set_global_client(mongodb_uri).await?;

    #[derive(Debug, Serialize, Deserialize, Model)]
    #[db("delete_example_db")]
    #[collection("users")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    // Insert users
    let users = vec![
        User { _id: None, name: "User1".into(), age: 20, active: false },
        User { _id: None, name: "User2".into(), age: 25, active: false },
        User { _id: None, name: "User3".into(), age: 30, active: true },
        User { _id: None, name: "User4".into(), age: 30, active: true },
    ];

    let mut inserted_ids = vec![];
    for user in users {
        let id = user.save().await?;
        inserted_ids.push(id);
    }

    // Delete all inactive users
    let result = User::delete(doc! { "active": false }).await?;
    println!("🗑️ Deleted {} inactive users", result.deleted_count);

    // Delete one active user
    let result = User::delete_one(doc! { "active": true }).await?;
    println!("🧹 Deleted {} active user(s)", result.deleted_count);

    // Delete last one by ID
    if let Some(id) = inserted_ids.pop() {
        let result = User::delete_by_id(id).await?;
        println!("❌ Deleted by ID: {}", result.deleted_count);
    }

    Ok(())
}
