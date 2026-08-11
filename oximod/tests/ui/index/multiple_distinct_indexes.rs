//! Compile-pass control for legitimate multiple index declarations.
//!
//! A model may declare several indexes — including one text index — as long
//! as at most one declaration is text-implying and all literal names are
//! distinct.

use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("test")]
#[collection("advanced")]
pub struct Advanced {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(text, weight = 5, default_language = "english", name = "text_idx")]
    title: String,

    #[index(unique, name = "user_idx")]
    user_id: String,

    #[index(sparse, order = -1, name = "age_idx")]
    age: i32,
}

fn main() {}
