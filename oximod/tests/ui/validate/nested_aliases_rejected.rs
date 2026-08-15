//! `dive`, `each`, and `items` are not aliases for `nested`; they remain
//! unknown validation keys.

use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Debug)]
#[model(embedded)]
struct Address {
    city: String,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[model(embedded)]
struct DiveAlias {
    #[validate(dive)]
    address: Address,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[model(embedded)]
struct EachAlias {
    #[validate(each)]
    addresses: Vec<Address>,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[model(embedded)]
struct ItemsAlias {
    #[validate(items)]
    addresses: Vec<Address>,
}

fn main() {}
