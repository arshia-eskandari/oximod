//! Use OxiMod model and typed-query operations inside MongoDB transactions.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example session_transactions
//! ```
//!
//! This example demonstrates how to:
//!
//! - create an instance-level `OxiClient` and obtain its MongoDB client;
//! - initialize declared indexes before starting transactional work;
//! - start, commit, and abort transactions with the MongoDB Rust driver;
//! - save models through an explicit `ClientSession`;
//! - use typed-query updates inside a transaction;
//! - read uncommitted writes through the same session;
//! - confirm that sessionless reads cannot see uncommitted writes;
//! - commit a multi-collection operation atomically;
//! - abort a multi-collection operation without leaving partial changes.
//!
//! OxiMod does not own the transaction lifecycle. Starting, committing,
//! aborting, and retrying transactions remain responsibilities of the
//! MongoDB Rust driver.
//!
//! Session participation is explicit. An ordinary OxiMod operation does not
//! automatically join an open transaction; every operation intended to be
//! atomic must receive the same `ClientSession`.
//!
//! Session-aware saves do not initialize declared indexes. Initialize indexes
//! before starting transactional work with `init_indexes()` or
//! `init_indexes_from()`.
//!
//! Transactions require a MongoDB deployment that supports them, such as a
//! replica set.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient, Queryable};
use serde::{Deserialize, Serialize};

// Inventory is stored separately from orders so a checkout operation needs
// multi-document atomicity.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("transaction_inventory")]
struct Inventory {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(unique, name = "inventory_sku_idx")]
    #[validate(non_empty)]
    sku: String,

    #[validate(non_negative)]
    available: i32,
}

// Orders live in their own collection.
//
// The unique reference demonstrates why indexes should be established before
// transactional writes begin.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("transaction_orders")]
struct Order {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(unique, name = "order_reference_idx")]
    #[validate(non_empty)]
    reference: String,

    #[validate(non_empty)]
    sku: String,

    #[validate(positive)]
    quantity: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    // Transactions are driven by the MongoDB client. Using an instance-level
    // OxiClient also demonstrates that session-aware typed queries do not
    // require OxiMod's global client.
    let oxi_client = OxiClient::new(mongodb_uri).await?;

    let client = oxi_client
        .client()
        .expect("a newly created OxiClient should contain a MongoDB client");

    // These collections belong only to the example, so clearing them keeps
    // repeated runs deterministic.
    Inventory::clear_from(client).await?;
    Order::clear_from(client).await?;

    // Session-aware saves deliberately do not create indexes because index
    // creation would be separate from the transaction. Establish all declared
    // indexes before transactional work begins.
    Inventory::init_indexes_from(client).await?;
    Order::init_indexes_from(client).await?;

    // Seed inventory outside the transaction.
    let inventory_id = Inventory::new()
        .sku("SKU-1")
        .available(5)
        .save_from(client)
        .await?;

    println!("Initial inventory: 5 units");

    committed_checkout(client, inventory_id).await?;
    aborted_checkout(client, inventory_id).await?;

    Ok(())
}

/// Performs a successful checkout.
///
/// The inventory update and order insertion either both commit or neither
/// becomes durable.
async fn committed_checkout(
    client: &mongodb::Client,
    inventory_id: ObjectId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("Committed transaction");
    println!("---------------------");

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    // Typed query terminals have `_with_session` counterparts. The filter and
    // update remain compile-time aware while the actual MongoDB operation runs
    // through this transaction's session.
    let reserved_inventory = Inventory::query()
        .filter(|inventory| inventory.sku.eq("SKU-1") & inventory.available.gt(0))
        .update_one_with_session(&mut session, |inventory| inventory.available.inc(-1))
        .await?
        .expect("SKU-1 should have inventory available");

    println!(
        "Inventory inside transaction after reservation: {}",
        reserved_inventory.available
    );

    // `save_with_session()` still performs OxiMod model validation before the
    // insert, but the insert itself participates in this transaction.
    let order_id = Order::new()
        .reference("ORDER-1")
        .sku("SKU-1")
        .quantity(1)
        .save_with_session(&mut session)
        .await?;

    // Reads through the same session observe the transaction's own writes.
    let order_inside_transaction = Order::find_by_id_with_session(order_id, &mut session)
        .await?
        .expect("the transaction should see its own inserted order");

    println!(
        "Order visible inside transaction: {}",
        order_inside_transaction.reference
    );

    let orders_inside_transaction = Order::query()
        .filter(|order| order.sku.eq("SKU-1"))
        .count_with_session(&mut session)
        .await?;

    println!("Orders visible inside transaction: {orders_inside_transaction}");

    // A read that does not receive the session is outside the transaction and
    // therefore cannot see the uncommitted order.
    let order_outside_transaction = Order::find_by_id_from(order_id, client).await?;

    println!(
        "Order visible outside transaction before commit: {}",
        order_outside_transaction.is_some()
    );

    session.commit_transaction().await?;

    // After commit, ordinary explicit-client OxiMod operations see both
    // changes.
    let committed_order = Order::find_by_id_from(order_id, client)
        .await?
        .expect("the committed order should now be visible");

    let committed_inventory = Inventory::find_by_id_from(inventory_id, client)
        .await?
        .expect("inventory should still exist");

    println!("Committed order: {}", committed_order.reference);
    println!(
        "Inventory after committed checkout: {}",
        committed_inventory.available
    );

    Ok(())
}

/// Performs the same kind of multi-collection work and then aborts it.
///
/// Neither the inventory decrement nor the new order survives.
async fn aborted_checkout(
    client: &mongodb::Client,
    inventory_id: ObjectId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("Aborted transaction");
    println!("-------------------");

    let inventory_before = Inventory::find_by_id_from(inventory_id, client)
        .await?
        .expect("inventory should exist before the transaction")
        .available;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let reserved_inventory = Inventory::query()
        .filter(|inventory| inventory.sku.eq("SKU-1") & inventory.available.gt(0))
        .update_one_with_session(&mut session, |inventory| inventory.available.inc(-1))
        .await?
        .expect("SKU-1 should still have inventory available");

    let aborted_order_id = Order::new()
        .reference("ORDER-2")
        .sku("SKU-1")
        .quantity(1)
        .save_with_session(&mut session)
        .await?;

    println!(
        "Inventory inside transaction before abort: {}",
        reserved_inventory.available
    );

    assert!(
        Order::exists_with_session(
            doc! {
                "_id": aborted_order_id,
            },
            &mut session,
        )
        .await?,
        "the transaction should see its own order"
    );

    // Nothing outside the session can see the new order yet.
    assert!(
        Order::find_by_id_from(aborted_order_id, client)
            .await?
            .is_none(),
        "the uncommitted order must not be visible outside the transaction"
    );

    session.abort_transaction().await?;

    // The inserted order disappeared with the aborted transaction.
    let aborted_order = Order::find_by_id_from(aborted_order_id, client).await?;

    // The inventory decrement was rolled back as well.
    let inventory_after = Inventory::find_by_id_from(inventory_id, client)
        .await?
        .expect("inventory should still exist after abort")
        .available;

    println!(
        "Aborted order exists after abort: {}",
        aborted_order.is_some()
    );
    println!("Inventory after abort: {inventory_after}");

    assert!(
        aborted_order.is_none(),
        "an order inserted in an aborted transaction must not persist"
    );

    assert_eq!(
        inventory_after, inventory_before,
        "an aborted transaction must restore the previous inventory value"
    );

    Ok(())
}
