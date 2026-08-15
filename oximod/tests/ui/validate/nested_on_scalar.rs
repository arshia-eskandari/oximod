//! `#[validate(nested)]` requires an embedded OxiMod model or a supported
//! container of one; a scalar field is rejected at compile time.

use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("nested_on_scalar")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(nested)]
    name: String,
}

fn main() {}
