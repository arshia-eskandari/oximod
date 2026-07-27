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
    let _query = User::query().filter(|user| user.age.matches_regex("^2"));
}
