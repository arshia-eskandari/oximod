//! Extract structured field errors from OxiMod validation failures.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example validate_extract_errors
//! ```
//!
//! This example demonstrates how to:
//!
//! - validate a model without connecting to MongoDB;
//! - distinguish validation failures from other OxiMod errors;
//! - access individual [`oximod::ValidationError`] values;
//! - group several messages under the same field;
//! - build a response shape suitable for an HTTP API or user interface;
//! - validate corrected input successfully.
//!
//! Validation is independent from persistence. An embedded model is used here
//! because the example represents submitted application data rather than an
//! independently stored MongoDB document.

use std::collections::BTreeMap;

use oximod::{Model, OxiModError, ValidationError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum AccountRole {
    Member,
    Manager,
}

// Submitted registration data.
//
// Embedded models receive generated constructors, setters, and `validate()`,
// but do not own a MongoDB collection.
#[derive(Debug, Serialize, Deserialize, Model)]
#[model(embedded)]
struct RegistrationInput {
    #[validate(min_length = 3, max_length = 20, alphanumeric)]
    username: String,

    #[validate(email)]
    email: String,

    #[validate(min = 18, max = 120)]
    age: i32,

    #[validate(non_empty)]
    bio: Option<String>,

    #[validate(pattern = r"^REF-[0-9]{4}$")]
    referral_code: Option<String>,

    #[validate(required)]
    role: Option<AccountRole>,
}

/// A transport-friendly representation of model validation failures.
///
/// A field maps to a vector rather than one string because OxiMod may report
/// more than one failed rule for the same field.
#[derive(Debug)]
struct ValidationResponse {
    success: bool,
    message: &'static str,
    errors: BTreeMap<String, Vec<String>>,
}

impl ValidationResponse {
    fn from_errors(errors: &[ValidationError]) -> Self {
        let mut grouped_errors = BTreeMap::<String, Vec<String>>::new();

        for error in errors {
            grouped_errors
                .entry(error.field.clone())
                .or_default()
                .push(error.message.clone());
        }

        Self {
            success: false,
            message: "The submitted registration is invalid.",
            errors: grouped_errors,
        }
    }

    fn print(&self) {
        println!("success: {}", self.success);
        println!("message: {}", self.message);
        println!("errors:");

        for (field, messages) in &self.errors {
            println!("  {field}:");

            for message in messages {
                println!("    - {message}");
            }
        }
    }
}

fn main() -> Result<(), OxiModError> {
    let invalid_input = RegistrationInput::new()
        .username("ab")
        .email("not-an-email")
        .age(16)
        .bio("   ")
        .referral_code("WELCOME");

    println!("Invalid registration");
    println!("--------------------");

    match invalid_input.validate() {
        Ok(()) => {
            println!("The invalid example unexpectedly passed.");
        }
        Err(error) => {
            // The accessor avoids coupling application code to the internal
            // layout of the `OxiModError::Validation` enum variant.
            let errors = error
                .validation_errors()
                .ok_or_else(|| OxiModError::custom("validation returned a non-validation error"))?;

            println!("OxiMod collected {} field-level errors.", errors.len());
            println!();

            let response = ValidationResponse::from_errors(errors);

            response.print();
        }
    }

    println!();
    println!("Corrected registration");
    println!("----------------------");

    let valid_input = RegistrationInput::new()
        .username("User1")
        .email("user1@example.com")
        .age(28)
        .bio("Backend developer learning OxiMod")
        .referral_code("REF-2048")
        .role(AccountRole::Member);

    valid_input.validate()?;

    println!("The corrected input passed validation.");

    // `validation_errors()` is selective: unrelated OxiMod errors do not
    // expose field-level validation details.
    let unrelated_error = OxiModError::custom("an unrelated application error");

    println!(
        "A custom error contains validation details: {}",
        unrelated_error.validation_errors().is_some()
    );

    Ok(())
}
