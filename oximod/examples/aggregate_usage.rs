//! Generate a sales summary with OxiMod's first-class aggregation API.
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
//! - start an aggregation with `Order::aggregate()`;
//! - filter with a typed `$match` stage built from generated fields;
//! - reshape the stream with a raw `$group` stage whose source field names
//!   stay compiler-linked through `raw_stage_with`;
//! - deserialize the reshaped output into a dedicated Rust type with
//!   `with_type`;
//! - execute with `all()` and print a deterministic report.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient, Queryable};
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
/// The `$group` stage replaces the order shape entirely, so the pipeline
/// output is deserialized into this dedicated type instead of `Order`.
/// MongoDB stores the grouping key in `_id`; Serde maps that field to the
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

    // The pipeline runs in exactly this stage order:
    //
    // 1. a typed `$match` keeps only paid orders — the field paths come from
    //    the generated fields, so a renamed or removed model field becomes a
    //    compile error;
    // 2. a raw `$group` produces one report row per category. Its stage body
    //    is ordinary MongoDB BSON, but `raw_stage_with` keeps the *source*
    //    field references (`$category`, `$quantity`, `$unit_price`)
    //    compiler-linked;
    // 3. a raw `$sort` orders by the computed revenue, which only exists in
    //    the pipeline stream, with the category key as a deterministic
    //    tiebreaker.
    //
    // After the `$group` stage the documents no longer look like `Order`, so
    // `with_type` selects the summary type the results deserialize into.
    let summaries = Order::aggregate()
        .match_(|order| order.status.eq("paid"))
        .raw_stage_with(|order| {
            doc! {
                "$group": {
                    "_id": format!("${}", order.category.name()),
                    "order_count": {
                        "$sum": 1,
                    },
                    "total_units": {
                        "$sum": format!("${}", order.quantity.name()),
                    },
                    "total_revenue": {
                        "$sum": {
                            "$multiply": [
                                format!("${}", order.quantity.name()),
                                format!("${}", order.unit_price.name()),
                            ],
                        },
                    },
                },
            }
        })
        .raw_stage(doc! {
            "$sort": {
                "total_revenue": -1,
                "_id": 1,
            },
        })
        .with_type::<CategorySummary>()
        .all()
        .await?;

    println!("Paid-order revenue by category");
    println!("-----------------------------------------------");
    println!(
        "{:<14} {:>8} {:>8} {:>11}",
        "Category", "Orders", "Units", "Revenue"
    );

    for summary in summaries {
        println!(
            "{:<14} {:>8} {:>8} ${:>10.2}",
            summary.category, summary.order_count, summary.total_units, summary.total_revenue,
        );
    }

    println!();
    println!("The pending order was intentionally excluded.");

    // For aggregation needs outside the builder — streaming cursors,
    // database-level pipelines, or driver features OxiMod does not wrap —
    // the raw MongoDB collection remains available:
    //
    //     let collection = Order::get_collection()?;
    //     let cursor = collection.aggregate(pipeline).await?;

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
