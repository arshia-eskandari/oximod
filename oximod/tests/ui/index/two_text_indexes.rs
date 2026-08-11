//! Compile-fail test for conflicting text index declarations.
//!
//! MongoDB allows at most one text index per collection. The second
//! declaration below uses a text-associated option (`weight`) without the
//! explicit `text` flag, so the test also verifies that declaration
//! validation uses the same text-implying predicate as index generation.

use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("test")]
#[collection("articles")]
struct Article {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(text)]
    title: String,

    #[index(weight = 3)]
    body: String,
}

fn main() {}
