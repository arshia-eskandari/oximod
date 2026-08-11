//! Compile-fail test for genuinely unknown struct attributes.
//!
//! Attributes that are neither derive helpers nor ordinary standard Rust
//! attributes remain rejected, and the diagnostic names the offending
//! attribute.

use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("test")]
#[collection("users")]
#[unknown]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    name: String,
}

fn main() {}
