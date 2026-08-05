//! Compile-fail test for `$elemMatch` on a scalar field.
//!
//! The test verifies that `elem_match()` is unavailable for an integer field.

use mongodb::bson::oid::ObjectId;
use oximod::{Model, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("test")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    age: i32,
}

fn main() {
    let _query = User::query().filter(|user| user.age.elem_match(|age| age.gte(18)));
}
