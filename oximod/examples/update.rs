//! Update documents safely with OxiMod's typed-query API.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example update
//! ```
//!
//! This example demonstrates how to:
//!
//! - select documents with typed filters;
//! - combine several update expressions with `&`;
//! - use `$set`, `$unset`, `$inc`, `$mul`, and `$addToSet`;
//! - receive the updated model from `update_one()`;
//! - update every matching document with `update_all()`;
//! - use sorting to select one document from several matches;
//! - handle an update that finds no matching document.
//!
//! Typed updates ensure that fields and update operators are compatible at
//! compile time. They do not run the model's validation rules after modifying
//! a stored document, so application code must still choose valid values.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiClient, OxiModError, Queryable};
use serde::{Deserialize, Serialize};

// A product stored as an independent inventory document.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("typed_update_inventory")]
struct InventoryItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(non_empty)]
    sku: String,

    #[validate(min_length = 2)]
    name: String,

    #[validate(non_negative)]
    stock: i32,

    #[validate(positive)]
    price: f64,

    #[default(true)]
    active: bool,

    #[default("available".to_string())]
    status: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,

    tags: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // This collection belongs only to the example, so clearing it keeps
    // repeated runs deterministic.
    InventoryItem::clear().await?;

    seed_inventory().await?;

    let initial_count = InventoryItem::query().count().await?;

    println!("Inventory items before updates: {initial_count}");

    println!();
    println!("Restock one product");
    println!("-------------------");

    // Update expressions using different MongoDB operators can be combined
    // with `&`. `update_one()` returns the document after the update.
    let restocked_lamp = InventoryItem::query()
        .filter(|item| item.sku.eq("OFF-003"))
        .update_one(|item| {
            item.stock.set(20)
                & item.status.set("available")
                & item.note.unset()
                & item.tags.add_to_set("restocked")
        })
        .await?
        .expect("OFF-003 should exist");

    println!(
        "{} now has {} units, status '{}', note {}, and tags [{}]",
        restocked_lamp.name,
        restocked_lamp.stock,
        restocked_lamp.status,
        display_note(&restocked_lamp),
        restocked_lamp.tags.join(", "),
    );

    println!();
    println!("Apply a promotion");
    println!("-----------------");

    // `$inc` accepts negative values for decrements, while `$mul` adjusts the
    // numeric value by a factor.
    let promoted_notebook = InventoryItem::query()
        .filter(|item| item.sku.eq("OFF-001"))
        .update_one(|item| {
            item.stock.inc(-2) & item.price.mul(0.9_f64) & item.tags.add_to_set("featured")
        })
        .await?
        .expect("OFF-001 should exist");

    println!(
        "{} now has {} units at ${:.2} with tags [{}]",
        promoted_notebook.name,
        promoted_notebook.stock,
        promoted_notebook.price,
        promoted_notebook.tags.join(", "),
    );

    println!();
    println!("Mark all active low-stock products");
    println!("----------------------------------");

    // Bulk updates accept typed filters but reject sorting, skipping, limits,
    // and pagination. Every matching product receives the same update.
    let low_stock_result = InventoryItem::query()
        .filter(|item| item.active.eq(true) & item.stock.lte(5))
        .update_all(|item| item.status.set("low-stock") & item.tags.add_to_set("restock"))
        .await?;

    println!(
        "Bulk update matched {} and modified {} products",
        low_stock_result.matched_count, low_stock_result.modified_count,
    );

    println!();
    println!("Prioritize one reorder");
    println!("----------------------");

    // More than one product now has the `low-stock` status. Sorting by stock
    // selects the product with the fewest remaining units.
    let priority_reorder = InventoryItem::query()
        .filter(|item| item.status.eq("low-stock"))
        .sort_by(|item| item.stock.asc())
        .then_sort_by(|item| item.sku.asc())
        .update_one(|item| item.status.set("reorder-now"))
        .await?
        .expect("at least one low-stock product should exist");

    println!(
        "Highest-priority reorder: {} with {} units",
        priority_reorder.name, priority_reorder.stock,
    );

    println!();
    println!("No-match update");
    println!("---------------");

    // A successful operation that matches no document returns `None`.
    let missing_update = InventoryItem::query()
        .filter(|item| item.sku.eq("MISSING-001"))
        .update_one(|item| item.active.set(false))
        .await?;

    println!(
        "Missing SKU produced an updated model: {}",
        missing_update.is_some()
    );

    println!();
    println!("Final inventory");
    println!("---------------");

    let final_inventory = InventoryItem::query()
        .sort_by(|item| item.sku.asc())
        .all()
        .await?;

    for item in final_inventory {
        print_item(&item);
    }

    Ok(())
}

/// Inserts a deterministic inventory used by the update workflow.
async fn seed_inventory() -> Result<(), OxiModError> {
    let items = [
        InventoryItem::new()
            .sku("OFF-001")
            .name("Notebook")
            .stock(10)
            .price(8.5)
            .note("seasonal display")
            .tags(vec!["office".to_string(), "paper".to_string()]),
        InventoryItem::new()
            .sku("OFF-002")
            .name("Pen Set")
            .stock(5)
            .price(12.0)
            .tags(vec!["office".to_string(), "writing".to_string()]),
        InventoryItem::new()
            .sku("OFF-003")
            .name("Desk Lamp")
            .stock(0)
            .price(45.0)
            .note("awaiting shipment")
            .tags(vec!["lighting".to_string()]),
        InventoryItem::new()
            .sku("OFF-004")
            .name("Old Binder")
            .stock(3)
            .price(6.0)
            .active(false)
            .status("discontinued")
            .note("clearance")
            .tags(vec!["archive".to_string()]),
        InventoryItem::new()
            .sku("OFF-005")
            .name("Marker Pack")
            .stock(2)
            .price(15.0)
            .tags(vec!["office".to_string(), "writing".to_string()]),
    ];

    for item in items {
        item.save().await?;
    }

    Ok(())
}

fn display_note(item: &InventoryItem) -> &str {
    item.note.as_deref().unwrap_or("not set")
}

fn print_item(item: &InventoryItem) {
    println!(
        "  {} | {:<11} | stock {:>2} | ${:>5.2} | {:<12} | [{}]",
        item.sku,
        item.name,
        item.stock,
        item.price,
        item.status,
        item.tags.join(", "),
    );
}
