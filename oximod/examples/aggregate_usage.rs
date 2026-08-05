//! Generate a sales summary with MongoDB's aggregation pipeline.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example aggregate_usage
//! ```
//!
//! This example demonstrates how to:
//!
//! - initialize OxiMod's global MongoDB client;
//! - construct and save models with generated fluent setters;
//! - use a generated field default;
//! - access the underlying typed MongoDB collection;
//! - build a raw aggregation pipeline with the MongoDB driver;
//! - deserialize aggregation results into a dedicated Rust type.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use futures_util::TryStreamExt;
use mongodb::bson::{doc, from_document, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

// An order stored in MongoDB.
//
// This is a collection model because each order is persisted as an
// independent MongoDB document.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("aggregate_orders")]
struct Order {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    customer: String,
    category: String,

    #[validate(positive)]
    quantity: i32,

    #[validate(positive)]
    unit_price: f64,

    /// Orders are pending unless the builder explicitly marks them as paid.
    #[default("pending".to_string())]
    status: String,
}

/// The shape produced by the aggregation pipeline.
///
/// MongoDB stores the grouping key in `_id`, so Serde maps that field to the
/// more descriptive Rust name `category`.
#[derive(Debug, Deserialize)]
struct CategorySummary {
    #[serde(rename = "_id")]
    category: String,
    order_count: i32,
    total_units: i32,
    total_revenue: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // This example owns its demonstration collection, so resetting it makes
    // repeated runs deterministic. Do not call `clear()` on production data.
    Order::clear().await?;

    seed_orders().await?;

    // OxiMod returns MongoDB's typed `Collection<Order>`. That collection can
    // use driver features directly when an operation is intentionally outside
    // OxiMod's higher-level helpers.
    let collection = Order::get_collection()?;

    let pipeline = vec![
        // Exclude pending orders from the revenue report.
        doc! {
            "$match": {
                "status": "paid",
            },
        },
        // Produce one report row per product category.
        doc! {
            "$group": {
                "_id": "$category",
                "order_count": {
                    "$sum": 1,
                },
                "total_units": {
                    "$sum": "$quantity",
                },
                "total_revenue": {
                    "$sum": {
                        "$multiply": [
                            "$quantity",
                            "$unit_price",
                        ],
                    },
                },
            },
        },
        // Display the highest-revenue category first. The secondary sort keeps
        // the output deterministic if two categories have equal revenue.
        doc! {
            "$sort": {
                "total_revenue": -1,
                "_id": 1,
            },
        },
    ];

    let mut cursor = collection.aggregate(pipeline).await?;

    println!("Paid-order revenue by category");
    println!("-----------------------------------------------");
    println!(
        "{:<14} {:>8} {:>8} {:>11}",
        "Category", "Orders", "Units", "Revenue"
    );

    while let Some(document) = cursor.try_next().await? {
        let summary: CategorySummary = from_document(document)?;

        println!(
            "{:<14} {:>8} {:>8} ${:>10.2}",
            summary.category, summary.order_count, summary.total_units, summary.total_revenue,
        );
    }

    println!();
    println!("The pending order was intentionally excluded.");

    Ok(())
}

/// Inserts a small deterministic dataset for the report.
async fn seed_orders() -> Result<(), oximod::OxiModError> {
    let orders = [
        Order::new()
            .customer("Customer1")
            .category("electronics")
            .quantity(1)
            .unit_price(120.0)
            .status("paid"),
        Order::new()
            .customer("Customer2")
            .category("electronics")
            .quantity(2)
            .unit_price(80.0)
            .status("paid"),
        Order::new()
            .customer("Customer3")
            .category("home")
            .quantity(3)
            .unit_price(25.0)
            .status("paid"),
        Order::new()
            .customer("Customer4")
            .category("books")
            .quantity(2)
            .unit_price(15.0)
            .status("paid"),
        Order::new()
            .customer("Customer5")
            .category("books")
            .quantity(1)
            .unit_price(30.0)
            .status("paid"),
        // `status` is omitted here, so the generated builder uses the
        // `#[default("pending".to_string())]` expression.
        Order::new()
            .customer("Customer6")
            .category("electronics")
            .quantity(1)
            .unit_price(500.0),
    ];

    for order in orders {
        order.save().await?;
    }

    Ok(())
}
