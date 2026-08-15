//! `nested` is a flag; assigning it a value is rejected during parsing.

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
    #[validate(nested = true)]
    address: Address,
}

fn main() {}
