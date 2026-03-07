//! Basic usage example for the oximod crate
//!
//! Run with: `cargo run --example basic_usage`
//!
//! This demonstrates how to:
//! - Connect to MongoDB
//! - Define a model with the `Model` derive macro
//! - Save a document using the builder API
//! - Count documents (raw MongoDB vs OxiMod helper)
//! - Check existence using both APIs

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load MongoDB URI
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in your .env file or environment");

    // Init global client
    OxiClient::init_global(mongodb_uri).await?;

    #[derive(Debug, Serialize, Deserialize, Model)]
    #[db("basic_usage_db")]
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

    // Insert user
    let user = User::new().name("User1").age(28);
    let id = user.save().await?;
    println!("✅ Saved user with _id: {}", id);

    // Get raw collection
    let collection = User::get_collection()?;

    // ---------------------------
    // COUNT (raw MongoDB)
    // ---------------------------

    let raw_count = collection.count_documents(doc! {}).await?;
    println!("📊 Raw count: {}", raw_count);

    // ---------------------------
    // COUNT (OxiMod helper)
    // ---------------------------

    let helper_count = User::count(doc! {}).await?;
    println!("⚡ Helper count: {}", helper_count);

    // Compare
    println!("Counts match: {}", raw_count == helper_count);

    // ---------------------------
    // EXISTS (raw MongoDB)
    // ---------------------------

    let raw_exists = collection
        .find_one(doc! { "name": "User1" })
        .await?
        .is_some();

    println!("🔎 Raw exists: {}", raw_exists);

    // ---------------------------
    // EXISTS (OxiMod helper)
    // ---------------------------

    let helper_exists = User::exists(doc! { "name": "User1" }).await?;

    println!("⚡ Helper exists: {}", helper_exists);

    println!("Exists match: {}", raw_exists == helper_exists);

    Ok(())
}
