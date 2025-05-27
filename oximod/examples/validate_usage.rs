//! Validation example for the oximod crate
//!
//! Run with: `cargo run --example validate_usage`
//!
//! This demonstrates how to:
//! - Connect to MongoDB
//! - Use the `Model` derive macro
//! - Apply validations like `min_length`, `email`, `positive`, `pattern`, etc.
//! - Use Rust enums instead of enum_values

use oximod::{ set_global_client, Model };
use mongodb::bson::oid::ObjectId;
use serde::{ Deserialize, Serialize };

#[derive(Debug, Serialize, Deserialize)]
enum Role {
    Admin,
    User,
    Guest,
}

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("validation_example_db")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(min_length = 3, max_length = 15)]
    username: String,

    #[validate(email)]
    email: Option<String>,

    #[validate(positive)]
    age: i32,

    #[validate(non_empty)]
    bio: Option<String>,

    #[validate(pattern = r"^SKU-\d{4}$")]
    sku: Option<String>,

    #[validate(non_negative)]
    points: i32,

    #[validate(required)]
    role: Option<Role>,

    #[default(false)]
    active: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env
        ::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in your .env file or environment");

    set_global_client(mongodb_uri).await?;

    // Clean up from previous runs
    User::clear().await?;

    println!("📥 Inserting a valid user...");
    let valid_user = User::new()
        .username("arshia".to_string())
        .email("arshia@example.com".to_string())
        .age(25)
        .bio("Rustacean and full-stack dev".to_string())
        .sku("SKU-1234".to_string())
        .points(0)
        .role(Role::User)
        .active(true);

    valid_user.save().await?;
    println!("✅ Valid user inserted successfully.");

    println!("⚠️ Inserting an invalid user...");
    let invalid_user = User::new()
        .username("ab".to_string()) // too short
        .email("not-an-email".to_string())
        .age(-1) // not positive
        .bio("   ".to_string()) // empty
        .sku("WRONGSKU".to_string()) // invalid pattern
        .points(-3); // not non-negative

    match invalid_user.save().await {
        Ok(_) => println!("❌ Unexpected success!"),
        Err(e) => {
            println!("🛑 Validation failed as expected:");
            println!("{:#}", e);
        }
    }

    Ok(())
}
