//! Validation error extraction example for the oximod crate
//!
//! Run with: `cargo run --example validate_extract_errors`
//!
//! This demonstrates how to:
//! - Connect to MongoDB
//! - Use the `Model` derive macro
//! - Validate a model without saving it
//! - Extract structured validation errors
//! - Simulate returning frontend-friendly error messages

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiClient, ValidationError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum Role {
    Admin,
    User,
    Guest,
}

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("validation_example_db")]
#[collection("users_extract_errors")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(min_length = 3, max_length = 15)]
    username: String,

    #[validate(email)]
    #[index(unique)]
    email: String,

    #[validate(positive)]
    age: i32,

    #[validate(non_empty)]
    bio: Option<String>,

    #[validate(pattern = r"^SKU-[0-9]{4}$")]
    sku: Option<String>,

    #[validate(non_negative)]
    points: i32,

    #[validate(required)]
    role: Option<Role>,

    #[default(false)]
    active: bool,
}

fn print_frontend_errors(errors: &[ValidationError]) {
    println!("📤 Simulated frontend response:");
    println!("{{");
    println!(r#"  "success": false,"#);
    println!(r#"  "message": "The submitted data is invalid.","#);
    println!(r#"  "errors": {{"#);

    for (index, error) in errors.iter().enumerate() {
        let comma = if index + 1 == errors.len() { "" } else { "," };
        println!(r#"    "{}": "{}"{}"#, error.field, error.message, comma);
    }

    println!("  }}");
    println!("}}");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in your .env file or environment");

    OxiClient::init_global(mongodb_uri).await?;

    User::clear().await?;

    println!("⚠️ Validating invalid user input...");

    let user = User::new()
        .username("ab") // too short
        .email("not-an-email")
        .age(-1) // not positive
        .bio("   ") // empty
        .sku("WRONGSKU") // invalid pattern
        .points(-3) // not non-negative
        .active(true);

    match user.validate() {
        Ok(()) => {
            println!("✅ Validation passed.");
            user.save().await?;
            println!("✅ User saved successfully.");
        }
        Err(error) => {
            if let Some(errors) = error.validation_errors() {
                println!("🛑 Validation failed:");
                println!("{error:#}");

                println!();
                print_frontend_errors(errors);
            } else {
                println!("❌ Unexpected error:");
                println!("{error:#}");
            }
        }
    }

    Ok(())
}
