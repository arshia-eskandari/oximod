//! Compile-fail test for nested `$elemMatch` on scalar arrays.
//!
//! The test verifies that `elem_match_nested()` is unavailable when an
//! array's elements are scalar values rather than embedded models.

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
    let _query = User::query().filter(|user| user.scores.elem_match_nested(|score| score.gte(80)));
}
