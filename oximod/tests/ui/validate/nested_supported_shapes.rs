//! Every supported `#[validate(nested)]` target shape compiles: a direct
//! embedded model, `Option`, `Vec`, `HashMap<String, _>`, recursive
//! combinations of those wrappers, and embedded-to-embedded edges.

use std::collections::HashMap;

use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Debug)]
#[model(embedded)]
struct Address {
    #[validate(non_empty)]
    city: String,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[model(embedded)]
struct Shipping {
    #[validate(nested)]
    address: Address,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("nested_supported_shapes")]
struct Order {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(nested)]
    address: Address,

    #[validate(required, nested)]
    optional_address: Option<Address>,

    #[validate(non_empty, nested)]
    addresses: Vec<Address>,

    #[validate(nested)]
    optional_each: Vec<Option<Address>>,

    #[validate(nested)]
    optional_all: Option<Vec<Address>>,

    #[validate(nested)]
    keyed: HashMap<String, Address>,

    #[validate(nested)]
    keyed_batches: HashMap<String, Vec<Option<Address>>>,

    #[validate(nested)]
    shipping: Shipping,
}

fn main() {
    let _ = Order::new();
}
