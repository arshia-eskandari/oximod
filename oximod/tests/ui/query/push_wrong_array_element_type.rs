//! Compile-fail test for mismatched array-update element types.
//!
//! The test verifies that `push()` accepts values compatible with the
//! array's declared element type.

use mongodb::bson::oid::ObjectId;
use oximod::{Model, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("test")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    scores: Vec<i32>,
}

fn main() {
    let _query = User::query().update_one(|user| user.scores.push("high"));
}
