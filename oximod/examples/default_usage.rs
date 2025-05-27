//! Default value example for the oximod crate
//!
//! Run with: `cargo run --example default_usage`
//!
//! This demonstrates how to:
//! - Use `#[default(...)]` to assign default values to fields
//! - Customize `_id` setter via `#[document_id_setter_ident(...)]`
//! - Use enum and scalar defaults
//! - Build a model using the fluent API

use oximod::{ set_global_client, Model };
use mongodb::bson::oid::ObjectId;
use serde::{ Deserialize, Serialize };

#[derive(Debug, Serialize, Deserialize)]
enum Status {
    Active,
    Inactive,
    Pending,
}

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("default_example_db")]
#[collection("users")]
#[document_id_setter_ident("with_mongo_id")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[default("Unnamed".to_string())]
    name: String,

    #[default(18)]
    age: i32,

    #[default(false)]
    verified: bool,

    #[default(Status::Pending)]
    status: Status,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    set_global_client(mongodb_uri).await?;

    // Clear any previous entries
    User::clear().await?;

    println!("📥 Inserting user with no custom values...");
    let default_user = User::default().save().await?;
    println!("✅ Saved user with _id: {}", default_user);

    println!("\n📥 Inserting customized user with fluent API...");
    let custom_user = User::default()
        .with_mongo_id(ObjectId::new())
        .name("User1".to_string())
        .age(30)
        .verified(true)
        .status(Status::Active)
        .save().await?;
    println!("✅ Saved custom user with _id: {}", custom_user);

    Ok(())
}
