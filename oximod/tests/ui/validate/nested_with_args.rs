//! `nested` is a flag; parenthesized arguments are rejected during parsing.

use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Debug)]
#[model(embedded)]
struct Address {
    city: String,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[model(embedded)]
struct Shipping {
    #[validate(nested(city))]
    address: Address,
}

fn main() {}
