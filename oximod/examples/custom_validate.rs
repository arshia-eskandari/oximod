//! Add reusable, domain-specific validation rules to a model.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example custom_validate
//! ```
//!
//! This example demonstrates how to:
//!
//! - define custom validators as ordinary Rust functions;
//! - validate string, numeric, and optional fields;
//! - combine custom validators with built-in validation rules;
//! - run validation explicitly with the generated `validate()` method;
//! - rely on `save()` to validate automatically before insertion;
//! - observe multiple field failures in one validation error.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

/// Domain-specific validators used by the model.
///
/// Custom validators are ordinary functions, so they can be organized,
/// unit-tested, and reused like any other application logic.
mod validators {
    /// Accepts usernames containing ASCII letters, digits, and underscores.
    ///
    /// The username must be 3–20 characters, contain no surrounding
    /// whitespace, and must not use a reserved administrative name.
    pub fn validate_username(value: &str) -> Result<(), &'static str> {
        if value.trim() != value {
            return Err("username cannot contain surrounding whitespace");
        }

        if !(3..=20).contains(&value.len()) {
            return Err("username must contain between 3 and 20 characters");
        }

        if !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err("username may contain only ASCII letters, digits, and underscores");
        }

        if ["admin", "administrator", "root"]
            .iter()
            .any(|reserved| value.eq_ignore_ascii_case(reserved))
        {
            return Err("username is reserved");
        }

        Ok(())
    }

    /// Restricts membership to realistic adult ages.
    pub fn validate_adult_age(value: &i32) -> Result<(), &'static str> {
        if !(18..=120).contains(value) {
            return Err("age must be between 18 and 120");
        }

        Ok(())
    }

    /// Accepts referral codes in the form `REF-1234`.
    pub fn validate_referral_code(value: &str) -> Result<(), &'static str> {
        let Some(digits) = value.strip_prefix("REF-") else {
            return Err("referral code must begin with `REF-`");
        };

        if digits.len() != 4 || !digits.chars().all(|character| character.is_ascii_digit()) {
            return Err("referral code must contain exactly four digits after `REF-`");
        }

        Ok(())
    }
}

// A membership record stored as an independent MongoDB document.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("custom_validation_members")]
struct Member {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(custom(crate::validators::validate_username))]
    username: String,

    #[validate(custom(crate::validators::validate_adult_age))]
    age: i32,

    /// Built-in and custom validators can coexist within the same model.
    #[validate(email)]
    email: Option<String>,

    /// The custom rule runs only when the optional field contains a value.
    #[validate(custom(crate::validators::validate_referral_code))]
    referral_code: Option<String>,

    #[default(true)]
    active: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // This collection belongs only to the example, so clearing it makes
    // repeated runs deterministic.
    Member::clear().await?;

    let valid_member = Member::new()
        .username("user_1")
        .age(28)
        .email("user1@example.com");

    // `referral_code` is `None`, so its optional custom validator is skipped.
    // Explicit validation is useful before beginning a larger workflow.
    valid_member.validate()?;

    println!("Preflight validation passed for user_1");

    // `save()` performs validation again before inserting the document.
    let member_id = valid_member.save().await?;

    println!("Saved user_1 with _id: {member_id}");

    let referred_member = Member::new()
        .username("user_2")
        .age(35)
        .email("user2@example.com")
        .referral_code("REF-2048");

    referred_member.validate()?;
    referred_member.save().await?;

    println!("Saved user_2 with referral code REF-2048");

    // This model violates several independent rules:
    //
    // - the username contains surrounding whitespace and is reserved;
    // - the member is younger than 18;
    // - the email is malformed;
    // - the referral code has the wrong format.
    //
    // OxiMod collects field failures rather than stopping at the first one.
    let invalid_member = Member::new()
        .username(" admin ")
        .age(16)
        .email("not-an-email")
        .referral_code("WELCOME");

    match invalid_member.save().await {
        Ok(id) => {
            println!("Unexpectedly saved invalid member with _id: {id}");
        }
        Err(error) => {
            println!("Rejected invalid member:");
            println!("{error}");
        }
    }

    // The rejected model was never inserted.
    let stored_members = Member::count(doc! {}).await?;

    println!("Stored members after rejected save: {stored_members}");

    Ok(())
}
