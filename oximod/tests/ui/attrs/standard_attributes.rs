//! Compile-pass control for ordinary standard struct attributes.
//!
//! Struct-level documentation, lint controls, and `#[non_exhaustive]` are
//! inert for the derive and must be accepted on both collection-backed and
//! embedded models.

use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

/// A documented collection model.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("test")]
#[collection("users")]
#[allow(dead_code)]
#[non_exhaustive]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    name: String,
}

/// A documented embedded model.
#[derive(Debug, Serialize, Deserialize, Model)]
#[model(embedded)]
#[allow(dead_code)]
#[non_exhaustive]
pub struct Address {
    street: String,
}

fn main() {
    let _user = User::new().name("User1");
    let _address = Address::new().street("13544 Cane St");
}
