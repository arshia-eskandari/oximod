//! Integration tests for explicit session and transaction support.
//!
//! These tests use an explicit `mongodb::Client` created from `MONGODB_URI`
//! instead of OxiMod's process-global client, so each test owns its session
//! directly. Transactions require the replica-set MongoDB test environment.

use mongodb::{Client, bson::doc, bson::oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

async fn client() -> Result<Client, Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
    Ok(Client::with_uri_str(uri).await?)
}

// Run test: cargo nextest run transactional_insert_rolls_back_on_abort
#[tokio::test]
async fn transactional_insert_rolls_back_on_abort() -> TestResult {
    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("insert_abort")]
    pub struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        sku: String,
        qty: i32,
    }

    let client = client().await?;
    Order::clear_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let id = Order::new()
        .sku("sku-1")
        .qty(2)
        .save_with_session(&mut session)
        .await?;

    // The same session observes its own uncommitted insert.
    let seen = Order::find_by_id_with_session(id, &mut session).await?;
    assert!(
        seen.is_some(),
        "session should see its own uncommitted write"
    );

    // A sessionless reader must not observe the uncommitted document.
    let outside = Order::find_by_id_from(id, &client).await?;
    assert!(
        outside.is_none(),
        "sessionless reader should not see an uncommitted write"
    );

    session.abort_transaction().await?;

    let after_abort = Order::find_by_id_from(id, &client).await?;
    assert!(after_abort.is_none(), "aborted insert should not persist");

    Ok(())
}

// Run test: cargo nextest run transactional_insert_persists_on_commit
#[tokio::test]
async fn transactional_insert_persists_on_commit() -> TestResult {
    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("insert_commit")]
    pub struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        sku: String,
        qty: i32,
    }

    let client = client().await?;
    Order::clear_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let mut order = Order::new().sku("sku-2").qty(1);
    let id = order.save_mut_with_session(&mut session).await?;

    session.commit_transaction().await?;

    let committed = Order::find_by_id_from(id, &client).await?;
    assert!(
        committed.is_some(),
        "committed insert should be visible outside the session"
    );

    Ok(())
}

// Run test: cargo nextest run multi_collection_writes_commit_atomically
#[tokio::test]
async fn multi_collection_writes_commit_atomically() -> TestResult {
    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("atomic_commit_orders")]
    pub struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        sku: String,
    }

    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("atomic_commit_inventory")]
    pub struct Inventory {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        sku: String,
        reserved: i32,
    }

    let client = client().await?;
    Order::clear_from(&client).await?;
    Inventory::clear_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let order_id = Order::new()
        .sku("sku-3")
        .save_with_session(&mut session)
        .await?;
    let inventory_id = Inventory::new()
        .sku("sku-3")
        .reserved(1)
        .save_with_session(&mut session)
        .await?;

    session.commit_transaction().await?;

    assert!(
        Order::find_by_id_from(order_id, &client).await?.is_some(),
        "order should survive commit"
    );
    assert!(
        Inventory::find_by_id_from(inventory_id, &client)
            .await?
            .is_some(),
        "inventory row should survive commit"
    );

    Ok(())
}

// Run test: cargo nextest run multi_collection_writes_abort_atomically
#[tokio::test]
async fn multi_collection_writes_abort_atomically() -> TestResult {
    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("atomic_abort_orders")]
    pub struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        sku: String,
    }

    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("atomic_abort_inventory")]
    pub struct Inventory {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        sku: String,
        reserved: i32,
    }

    let client = client().await?;
    Order::clear_from(&client).await?;
    Inventory::clear_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    Order::new()
        .sku("sku-4")
        .save_with_session(&mut session)
        .await?;
    Inventory::new()
        .sku("sku-4")
        .reserved(1)
        .save_with_session(&mut session)
        .await?;

    session.abort_transaction().await?;

    assert_eq!(
        Order::count_from(doc! {}, &client).await?,
        0,
        "no order should survive abort"
    );
    assert_eq!(
        Inventory::count_from(doc! {}, &client).await?,
        0,
        "no inventory row should survive abort"
    );

    Ok(())
}

// Run test: cargo nextest run transactional_update_by_id_rolls_back_on_abort
#[tokio::test]
async fn transactional_update_by_id_rolls_back_on_abort() -> TestResult {
    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("update_abort")]
    pub struct Seat {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        row: String,
        booked: bool,
    }

    let client = client().await?;
    Seat::clear_from(&client).await?;

    let id = Seat::new()
        .row("A1")
        .booked(false)
        .save_from(&client)
        .await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let result =
        Seat::update_by_id_with_session(id, doc! { "$set": { "booked": true } }, &mut session)
            .await?;
    assert_eq!(result.modified_count, 1);

    // Read-your-own-writes: the session sees the updated value.
    let in_session = Seat::find_by_id_with_session(id, &mut session)
        .await?
        .expect("document should exist inside the transaction");
    assert!(in_session.booked, "session should see its own update");

    // A sessionless reader still sees the pre-transaction value.
    let outside = Seat::find_by_id_from(id, &client)
        .await?
        .expect("document should exist outside the transaction");
    assert!(
        !outside.booked,
        "sessionless reader should not see the uncommitted update"
    );

    session.abort_transaction().await?;

    let after_abort = Seat::find_by_id_from(id, &client)
        .await?
        .expect("document should still exist after abort");
    assert!(!after_abort.booked, "aborted update should roll back");

    Ok(())
}

// Run test: cargo nextest run transactional_update_by_id_persists_on_commit
#[tokio::test]
async fn transactional_update_by_id_persists_on_commit() -> TestResult {
    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("update_commit")]
    pub struct Seat {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        row: String,
        booked: bool,
    }

    let client = client().await?;
    Seat::clear_from(&client).await?;

    let id = Seat::new()
        .row("B2")
        .booked(false)
        .save_from(&client)
        .await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    Seat::update_by_id_with_session(id, doc! { "$set": { "booked": true } }, &mut session).await?;

    session.commit_transaction().await?;

    let committed = Seat::find_by_id_from(id, &client)
        .await?
        .expect("document should exist after commit");
    assert!(committed.booked, "committed update should persist");

    Ok(())
}

// Run test: cargo nextest run transactional_delete_by_id_rolls_back_on_abort
#[tokio::test]
async fn transactional_delete_by_id_rolls_back_on_abort() -> TestResult {
    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("delete_abort")]
    pub struct Booking {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        code: String,
    }

    let client = client().await?;
    Booking::clear_from(&client).await?;

    let id = Booking::new().code("BK-1").save_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let result = Booking::delete_by_id_with_session(id, &mut session).await?;
    assert_eq!(result.deleted_count, 1);

    // The session no longer sees the deleted document.
    assert!(
        Booking::find_by_id_with_session(id, &mut session)
            .await?
            .is_none(),
        "session should see its own delete"
    );

    session.abort_transaction().await?;

    assert!(
        Booking::find_by_id_from(id, &client).await?.is_some(),
        "aborted delete should roll back"
    );

    Ok(())
}

// Run test: cargo nextest run session_aware_exists_count_and_clear_respect_the_transaction
#[tokio::test]
async fn session_aware_exists_count_and_clear_respect_the_transaction() -> TestResult {
    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("exists_count_clear")]
    pub struct Entry {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        label: String,
    }

    let client = client().await?;
    Entry::clear_from(&client).await?;

    Entry::new().label("kept-1").save_from(&client).await?;
    Entry::new().label("kept-2").save_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    Entry::new()
        .label("uncommitted")
        .save_with_session(&mut session)
        .await?;

    // Session-aware reads observe the uncommitted insert.
    assert!(
        Entry::exists_with_session(doc! { "label": "uncommitted" }, &mut session).await?,
        "exists_with_session should see the uncommitted write"
    );
    assert_eq!(
        Entry::count_with_session(doc! {}, &mut session).await?,
        3,
        "count_with_session should include the uncommitted write"
    );

    // Sessionless reads do not.
    assert!(
        !Entry::exists_from(doc! { "label": "uncommitted" }, &client).await?,
        "exists_from should not see the uncommitted write"
    );
    assert_eq!(
        Entry::count_from(doc! {}, &client).await?,
        2,
        "count_from should not include the uncommitted write"
    );

    // clear_with_session participates in the transaction.
    let cleared = Entry::clear_with_session(&mut session).await?;
    assert_eq!(cleared.deleted_count, 3);
    assert_eq!(Entry::count_with_session(doc! {}, &mut session).await?, 0);

    session.abort_transaction().await?;

    // The abort rolls back both the insert and the clear.
    assert_eq!(
        Entry::count_from(doc! {}, &client).await?,
        2,
        "aborted transaction should leave the collection untouched"
    );

    Ok(())
}

// Run test: cargo nextest run ordinary_operations_do_not_join_an_open_transaction
#[tokio::test]
async fn ordinary_operations_do_not_join_an_open_transaction() -> TestResult {
    #[derive(Model, Serialize, Deserialize)]
    #[db("session_tx_test")]
    #[collection("negative_control")]
    pub struct Event {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        kind: String,
    }

    let client = client().await?;
    Event::clear_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    Event::new()
        .kind("transactional")
        .save_with_session(&mut session)
        .await?;

    // A plain OxiMod call made while the transaction is open does not join it:
    // it commits independently and is immediately visible to other readers.
    let plain_id = Event::new().kind("plain").save_from(&client).await?;
    assert!(
        Event::find_by_id_from(plain_id, &client).await?.is_some(),
        "a non-session save should commit independently while a transaction is open"
    );

    session.abort_transaction().await?;

    // The abort removes only the transactional write.
    assert!(
        Event::find_by_id_from(plain_id, &client).await?.is_some(),
        "the independent write should survive the abort"
    );
    assert_eq!(
        Event::count_from(doc! { "kind": "transactional" }, &client).await?,
        0,
        "the transactional write should be rolled back"
    );

    Ok(())
}
