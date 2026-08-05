//! Compile-fail test for array updates on scalar fields.
//!
//! The test verifies that `push()` is unavailable for a `String` field.

use mongodb::bson::oid::ObjectId;
use oximod::{Model, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("test")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    name: String,
}

fn main() {
    let _query = User::query().update_one(|user| user.name.push("mongodb"));
}
