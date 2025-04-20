//! Aggregation example for the oximod crate
//!
//! Run with: `cargo run --example aggregate_usage`
//!
//! This demonstrates how to:
//! - Connect to MongoDB
//! - Define a model with the `Model` derive macro
//! - Insert multiple documents
//! - Perform an aggregation query on a collection

use oximod::{set_global_client, Model};
use mongodb::bson::{doc, oid::ObjectId, Bson};
use serde::{Deserialize, Serialize};
use futures_util::stream::StreamExt;

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
    #[db("aggregate_example_db")]
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

    // Insert some users
    let users = vec![
        User { _id: None, name: "Alice".into(), age: 30, active: true },
        User { _id: None, name: "Bob".into(), age: 25, active: true },
        User { _id: None, name: "Charlie".into(), age: 30, active: false },
    ];

    for user in users {
        user.save().await?;
    }

    // Define an aggregation pipeline to group by age and count
    let pipeline = vec![
        doc! {
            "$group": {
                "_id": "$age",
                "count": { "$sum": 1 }
            }
        },
        doc! {
            "$sort": { "count": -1 }
        },
    ];

    // Run the aggregation
    let mut cursor = User::aggregate(pipeline).await?;
    println!("📊 Aggregation results by age:");

    while let Some(doc) = cursor.next().await {
        let doc = doc?;
        let age = doc.get("_id").unwrap_or(&Bson::Null);
        let count = doc.get("count").unwrap_or(&Bson::Null);
        println!("🧓 Age: {}, 👥 Count: {}", age, count);
    }

    Ok(())
}
