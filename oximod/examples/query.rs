//! Query OxiMod models through MongoDB's raw collection API.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example query
//! ```
//!
//! This example demonstrates how to:
//!
//! - obtain a typed MongoDB collection from an OxiMod model;
//! - write compound filters with `bson::doc!`;
//! - apply MongoDB operators such as `$lte` and `$exists`;
//! - sort and limit a raw `find()` operation;
//! - collect a MongoDB cursor into typed model values;
//! - retrieve one optional result with `find_one()`;
//! - count matching documents;
//! - retrieve distinct field values;
//! - account for Serde-renamed field names in raw BSON documents.
//!
//! Raw collection access is useful when an operation is not represented by
//! OxiMod's typed-query builder or when direct control over the MongoDB driver
//! is desirable. The tradeoff is that field names and operators are written as
//! strings and are therefore not checked by Rust.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use futures_util::TryStreamExt;
use mongodb::bson::{Bson, doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

// A product stored as an independent MongoDB document.
//
// `rename_all = "camelCase"` is intentional: it demonstrates that raw query
// documents must use serialized MongoDB names rather than Rust field names.
#[derive(Debug, Serialize, Deserialize, Model)]
#[serde(rename_all = "camelCase")]
#[db("oximod_examples")]
#[collection("raw_query_products")]
struct Product {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(non_empty)]
    sku: String,

    #[validate(min_length = 2)]
    name: String,

    #[validate(non_empty)]
    category: String,

    #[validate(positive)]
    price: f64,

    #[default(true)]
    in_stock: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(positive)]
    sale_price: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // This collection belongs only to the example, so clearing it keeps
    // repeated runs deterministic.
    Product::clear().await?;

    seed_products().await?;

    // The collection remains typed as `Collection<Product>`. Driver reads
    // therefore deserialize matching documents directly into `Product`.
    let collection = Product::get_collection()?;

    println!("Collection: {}", collection.name());

    println!();
    println!("Available office products under $100");
    println!("------------------------------------");

    // Raw BSON filters use serialized MongoDB field names. Because Serde
    // renamed `in_stock` to camelCase, the filter must say `inStock`.
    //
    // Query options are chained onto the driver's find action before `.await`.
    let cursor = collection
        .find(doc! {
            "category": "office",
            "inStock": true,
            "price": {
                "$lte": 100.0,
            },
        })
        .sort(doc! {
            "price": 1,
            "name": 1,
        })
        .limit(3)
        .await?;

    let affordable_products: Vec<Product> = cursor.try_collect().await?;

    for product in &affordable_products {
        println!(
            "  {:<12} {:<18} ${:>6.2}",
            product.sku, product.name, product.price
        );
    }

    println!();
    println!("Exact SKU lookup");
    println!("----------------");

    // `find_one()` returns `Option<Product>` because a successful query may
    // legitimately find no matching document.
    let chair = collection
        .find_one(doc! {
            "sku": "OFF-002",
        })
        .await?;

    match chair {
        Some(product) => {
            println!(
                "  {} costs ${:.2} and is in stock: {}",
                product.name, product.price, product.in_stock
            );
        }
        None => {
            println!("  No product with SKU OFF-002 was found");
        }
    }

    println!();
    println!("Optional-field query");
    println!("--------------------");

    // `sale_price` is serialized as `salePrice` and omitted when it is `None`.
    // `$exists` therefore finds only documents carrying an actual sale price.
    let discounted_cursor = collection
        .find(doc! {
            "salePrice": {
                "$exists": true,
            },
        })
        .sort(doc! {
            "salePrice": 1,
            "name": 1,
        })
        .await?;

    let discounted_products: Vec<Product> = discounted_cursor.try_collect().await?;

    for product in &discounted_products {
        let sale_price = product
            .sale_price
            .expect("the `$exists` filter should return a sale price");

        println!(
            "  {:<18} ${:>6.2} instead of ${:>6.2}",
            product.name, sale_price, product.price
        );
    }

    println!();
    println!("Counts and distinct values");
    println!("--------------------------");

    let available_count = collection
        .count_documents(doc! {
            "inStock": true,
        })
        .await?;

    println!("  Products currently in stock: {available_count}");

    // `distinct()` returns BSON values because MongoDB fields need not have one
    // uniform type. This model guarantees strings, so the example extracts
    // only string values and sorts them for deterministic output.
    let category_values = collection
        .distinct(
            "category",
            doc! {
                "inStock": true,
            },
        )
        .await?;

    let mut categories: Vec<String> = category_values
        .into_iter()
        .filter_map(|value| match value {
            Bson::String(category) => Some(category),
            _ => None,
        })
        .collect();

    categories.sort();

    println!(
        "  Categories containing available products: {}",
        categories.join(", ")
    );

    Ok(())
}

/// Inserts a deterministic product catalogue for the query examples.
async fn seed_products() -> Result<(), oximod::OxiModError> {
    let products = [
        Product::new()
            .sku("OFF-001")
            .name("Desk Lamp")
            .category("office")
            .price(45.0),
        Product::new()
            .sku("OFF-002")
            .name("Ergonomic Chair")
            .category("office")
            .price(220.0)
            .sale_price(180.0),
        Product::new()
            .sku("OFF-003")
            .name("Notebook")
            .category("office")
            .price(8.5)
            .sale_price(6.5),
        Product::new()
            .sku("OFF-004")
            .name("Standing Desk")
            .category("office")
            .price(350.0)
            .in_stock(false),
        Product::new()
            .sku("KIT-001")
            .name("Water Bottle")
            .category("kitchen")
            .price(20.0),
        Product::new()
            .sku("OFF-005")
            .name("Pen Set")
            .category("office")
            .price(12.0),
    ];

    for product in products {
        product.save().await?;
    }

    Ok(())
}
