//! A first end-to-end OxiMod workflow.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example basic_usage
//! ```
//!
//! This example demonstrates how to:
//!
//! - define a collection-backed model;
//! - initialize OxiMod's global MongoDB client;
//! - construct models with generated fluent setters;
//! - use generated defaults;
//! - validate and save documents;
//! - find a document by its MongoDB object ID;
//! - count documents and check whether matches exist;
//! - access the underlying typed MongoDB collection.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

// A user stored as an independent MongoDB document.
//
// OxiMod collection models require both `#[db(...)]` and
// `#[collection(...)]`.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("basic_users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(min_length = 2)]
    name: String,

    #[validate(email)]
    email: Option<String>,

    /// New users are active unless the builder explicitly says otherwise.
    #[default(true)]
    active: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    // Global initialization enables convenience methods such as `save()`,
    // `find_by_id()`, `count()`, and `exists()`.
    OxiClient::init_global(mongodb_uri).await?;

    // This collection belongs to the example, so resetting it makes repeated
    // runs deterministic. Never clear a production collection casually.
    User::clear().await?;

    let user1 = User::new().name("User1").email("user1@example.com");

    let user2 = User::new().name("User2").active(false);

    // `save()` validates the model before inserting it. MongoDB generates the
    // `_id` because neither model supplied one.
    let user1_id = user1.save().await?;
    let user2_id = user2.save().await?;

    println!("Saved User1 with _id: {user1_id}");
    println!("Saved User2 with _id: {user2_id}");

    // ID helpers return `Option<User>` because a syntactically valid object ID
    // does not necessarily identify an existing document.
    let saved_user1 = User::find_by_id(user1_id)
        .await?
        .expect("User1 should exist after being saved");

    println!(
        "Found by ID: {} (active: {})",
        saved_user1.name, saved_user1.active
    );

    let total_users = User::count(doc! {}).await?;
    let active_users = User::count(doc! { "active": true }).await?;

    println!("Total users: {total_users}");
    println!("Active users: {active_users}");

    let user1_has_email = User::exists(doc! { "email": "user1@example.com" }).await?;

    let missing_user_exists = User::exists(doc! { "name": "MissingUser" }).await?;

    println!("User1 email exists: {user1_has_email}");
    println!("Missing user exists: {missing_user_exists}");

    // OxiMod does not hide the MongoDB driver. `get_collection()` returns a
    // typed `mongodb::Collection<User>` for operations that are not covered by
    // a higher-level helper.
    let collection = User::get_collection()?;

    let inactive_user = collection
        .find_one(doc! { "active": false })
        .await?
        .expect("User2 should be the inactive user");

    println!(
        "Inactive user from typed collection: {}",
        inactive_user.name
    );

    Ok(())
}
