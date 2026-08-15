//! `#[validate(nested)]` descends into `#[model(embedded)]` models only.
//! A collection-backed model is not a nested-validation leaf.

use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("nested_profiles")]
struct Profile {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    name: String,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("nested_users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(nested)]
    profile: Profile,
}

fn main() {}
