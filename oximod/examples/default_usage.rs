//! Define practical model defaults while keeping every value overridable.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example default_usage
//! ```
//!
//! This example demonstrates how to:
//!
//! - initialize fields with `#[default(...)]`;
//! - use literal, function-call, macro, and enum default expressions;
//! - rely on `Default::default()` for fields without an explicit default;
//! - construct equivalent models with `new()` and `Default::default()`;
//! - override any generated default through fluent setters;
//! - inspect the values that were persisted to MongoDB.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Serialize, Deserialize)]
enum DigestFrequency {
    Daily,
    Weekly,
    Never,
}

// Application preferences stored as one independent MongoDB document per user.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("default_preferences")]
struct UserPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    // This field has no configured default. If the setter is omitted, the
    // generated constructor uses `String::default()`, which is an empty string.
    user_name: String,

    // A string-producing function call can be used as the default expression.
    #[default(String::from("en-CA"))]
    language: String,

    // Literal defaults work for ordinary scalar fields.
    #[default(true)]
    email_notifications: bool,

    #[default(25_u32)]
    items_per_page: u32,

    // Enum variants can be used directly.
    #[default(Theme::System)]
    theme: Theme,

    #[default(DigestFrequency::Weekly)]
    digest_frequency: DigestFrequency,

    // Macro expressions are also supported.
    #[default(vec!["announcements".to_string(), "security".to_string()])]
    subscribed_topics: Vec<String>,

    // No explicit default means `Option::default()`, which is `None`.
    time_zone: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // This collection belongs only to the example, so clearing it keeps
    // repeated runs deterministic.
    UserPreferences::clear().await?;

    // `new()` initializes every field. Configured expressions are used where
    // present; all other fields use their type's `Default` implementation.
    let default_preferences = UserPreferences::new().user_name("User1");

    println!("Defaults before saving User1:");
    print_preferences(&default_preferences);

    let user1_id = default_preferences.save().await?;

    // `Default::default()` delegates to the same generated constructor as
    // `new()`. Fluent setters then replace selected values.
    let customized_preferences = UserPreferences::default()
        .user_name("User2")
        .language("fr-CA")
        .email_notifications(false)
        .items_per_page(50_u32)
        .theme(Theme::Dark)
        .digest_frequency(DigestFrequency::Daily)
        .subscribed_topics(vec!["product".to_string(), "community".to_string()])
        .time_zone("America/Toronto");

    println!();
    println!("Customized values before saving User2:");
    print_preferences(&customized_preferences);

    let user2_id = customized_preferences.save().await?;

    // Re-fetching proves that the generated defaults and setter overrides were
    // serialized and stored rather than existing only in the local builders.
    let stored_user1 = UserPreferences::find_by_id(user1_id)
        .await?
        .expect("User1 preferences should exist after saving");

    let stored_user2 = UserPreferences::find_by_id(user2_id)
        .await?
        .expect("User2 preferences should exist after saving");

    println!();
    println!("Values read back from MongoDB:");
    print_preferences(&stored_user1);
    println!();
    print_preferences(&stored_user2);

    Ok(())
}

fn print_preferences(preferences: &UserPreferences) {
    println!("  user: {}", preferences.user_name);
    println!("  language: {}", preferences.language);
    println!("  email notifications: {}", preferences.email_notifications);
    println!("  items per page: {}", preferences.items_per_page);
    println!("  theme: {:?}", preferences.theme);
    println!("  digest frequency: {:?}", preferences.digest_frequency);
    println!(
        "  subscribed topics: {}",
        preferences.subscribed_topics.join(", ")
    );
    println!(
        "  time zone: {}",
        preferences.time_zone.as_deref().unwrap_or("not set")
    );
}
