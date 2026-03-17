//! Basic usage example for the oximod crate with a custom validator
//!
//! Run with: `cargo run --example custom_validate`
//!
//! This demonstrates how to:
//! - Connect to MongoDB
//! - Define a model with the `Model` derive macro
//! - Use `#[validate(custom(...))]` on a field
//! - Save a document using the builder API
//! - Count documents (raw MongoDB vs OxiMod helper)
//! - Check existence using both APIs

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

mod validators {
    pub fn validate_name(value: &String) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("name cannot be empty".into());
        }

        if value.len() < 3 {
            return Err("name must be at least 3 characters long".into());
        }

        if value == "admin" {
            return Err("name 'admin' is reserved".into());
        }

        Ok(())
    }

    pub fn validate_age(value: &i32) -> Result<(), String> {
        if *value < 18 {
            return Err("age must be at least 18".into());
        }

        if *value > 120 {
            return Err("age must be realistic".into());
        }

        Ok(())
    }
}

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

        #[validate(custom(crate::validators::validate_name))]
        name: String,

        #[validate(custom(crate::validators::validate_age))]
        age: i32,

        #[default(true)]
        active: bool,
    }

    // Clean up previous runs
    User::clear().await?;

    // Insert valid user
    let user = User::new().name("User1").age(28);
    let id = user.save().await?;
    println!("✅ Saved user with _id: {}", id);

    // Try invalid user
    let invalid_user = User::new().name("ad").age(16);

    match invalid_user.save().await {
        Ok(id) => println!("Unexpectedly saved invalid user with _id: {}", id),
        Err(err) => println!("❌ Validation failed as expected: {}", err),
    }

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
