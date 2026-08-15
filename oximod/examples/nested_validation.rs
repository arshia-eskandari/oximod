//! Validate embedded models through their parent with `#[validate(nested)]`.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example nested_validation
//! ```
//!
//! This example demonstrates how to:
//!
//! - opt a containing field into nested validation with `#[validate(nested)]`;
//! - descend through a bare embedded field and a `Vec` of embedded models;
//! - read path-aware validation errors such as `address.postal_code` and
//!   `previous_addresses[1].city`;
//! - keep the no-descent behavior on fields without the attribute.
//!
//! Nested validation runs anywhere whole-model validation already runs, so
//! the same rejections apply to `save()` and the other validated persistence
//! paths. This example only calls `validate()` and needs no database.

use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[model(embedded)]
struct Address {
    #[validate(non_empty)]
    city: String,

    #[validate(pattern = r"^[0-9]{5}$")]
    postal_code: String,
}

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("app")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(non_empty)]
    name: String,

    // Descends into the embedded value.
    #[validate(nested)]
    address: Address,

    // Descends into every element in ascending index order.
    #[validate(nested)]
    previous_addresses: Vec<Address>,

    // No `nested` attribute: this field keeps the no-descent behavior, and
    // its rules are only checked when calling `backup_address.validate()`
    // directly.
    backup_address: Address,
}

fn main() {
    let user = User::new()
        .name("User1")
        .address(Address::new().city("Springfield").postal_code("bad"))
        .previous_addresses(vec![
            Address::new().city("Shelbyville").postal_code("54321"),
            Address::new().city("").postal_code("11111"),
        ])
        .backup_address(Address::new().city("").postal_code("also-bad"));

    match user.validate() {
        Ok(()) => println!("unexpectedly valid"),
        Err(error) => {
            // Prints, in deterministic order:
            //
            //   address.postal_code: does not match the required pattern
            //   previous_addresses[1].city: must be non-empty
            //
            // The `backup_address` violations are not listed because that
            // field did not opt into nested validation.
            for failure in error.validation_errors().unwrap_or_default() {
                println!("{}: {}", failure.field, failure.message);
            }
        }
    }
}
