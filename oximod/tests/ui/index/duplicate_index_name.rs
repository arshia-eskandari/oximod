//! Compile-fail test for duplicate literal index names.
//!
//! MongoDB rejects `createIndexes` when two declared indexes share a name, so
//! duplicate literal `#[index(name = "...")]` values on one model are
//! reported at compile time.

use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("test")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(unique, name = "shared_idx")]
    name: String,

    #[index(sparse, name = "shared_idx")]
    age: i32,
}

fn main() {}
