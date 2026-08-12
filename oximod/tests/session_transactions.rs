//! Integration tests for explicit session and transaction support.
//!
//! These tests use an explicit `mongodb::Client` created from `MONGODB_URI`
//! instead of OxiMod's process-global client, so each test owns its session
//! directly. Transactions require the replica-set MongoDB test environment.

use mongodb::{Client, bson::doc, bson::oid::ObjectId};
use oximod::{Model, Queryable};
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

// Run test: cargo nextest run typed_first_with_session_applies_sort_and_sees_uncommitted_writes
#[tokio::test]
async fn typed_first_with_session_applies_sort_and_sees_uncommitted_writes() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("typed_first")]
    pub struct Task {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        priority: i32,
    }

    let client = client().await?;
    Task::clear_from(&client).await?;

    Task::new()
        .name("low")
        .priority(5)
        .save_from(&client)
        .await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    Task::new()
        .name("urgent")
        .priority(1)
        .save_with_session(&mut session)
        .await?;

    // Sorting selects the uncommitted document only inside the session.
    let first = Task::query()
        .sort_by(|task| task.priority.asc())
        .first_with_session(&mut session)
        .await?
        .expect("a task should match");
    assert_eq!(first.name, "urgent");

    session.abort_transaction().await?;

    Ok(())
}

// Run test: cargo nextest run typed_all_with_session_iterates_session_cursor_with_modifiers
#[tokio::test]
async fn typed_all_with_session_iterates_session_cursor_with_modifiers() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("typed_all")]
    pub struct Item {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        rank: i32,
        active: bool,
    }

    let client = client().await?;
    Item::clear_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    for rank in 1..=5 {
        Item::new()
            .rank(rank)
            .active(rank != 4)
            .save_with_session(&mut session)
            .await?;
    }

    // Filter, sort, skip, and limit apply to the session cursor: the active
    // ranks are [1, 2, 3, 5]; skipping one and taking two yields [2, 3].
    let items = Item::query()
        .filter(|item| item.active.eq(true))
        .sort_by(|item| item.rank.asc())
        .skip(1)
        .limit(2)
        .all_with_session(&mut session)
        .await?;

    let ranks: Vec<i32> = items.iter().map(|item| item.rank).collect();
    assert_eq!(ranks, vec![2, 3]);

    // A sessionless reader sees none of the uncommitted documents.
    assert_eq!(Item::count_from(doc! {}, &client).await?, 0);

    session.abort_transaction().await?;

    Ok(())
}

// Run test: cargo nextest run typed_count_with_session_composes_filters
#[tokio::test]
async fn typed_count_with_session_composes_filters() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("typed_count")]
    pub struct Vote {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        topic: String,
        weight: i32,
    }

    let client = client().await?;
    Vote::clear_from(&client).await?;

    Vote::new().topic("a").weight(1).save_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    Vote::new()
        .topic("a")
        .weight(3)
        .save_with_session(&mut session)
        .await?;
    Vote::new()
        .topic("b")
        .weight(3)
        .save_with_session(&mut session)
        .await?;

    // Composed filters apply and the count includes the session's own writes.
    let count = Vote::query()
        .filter(|vote| vote.topic.eq("a"))
        .filter(|vote| vote.weight.gte(1))
        .count_with_session(&mut session)
        .await?;
    assert_eq!(count, 2);

    session.abort_transaction().await?;

    Ok(())
}

// Run test: cargo nextest run typed_update_one_with_session_applies_array_filters_and_rolls_back
#[tokio::test]
async fn typed_update_one_with_session_applies_array_filters_and_rolls_back() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
    pub struct Address {
        city: String,
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("session_tx_test")]
    #[collection("typed_update_one")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        addresses: Vec<Address>,
    }

    let client = client().await?;
    User::clear_from(&client).await?;

    let id = User::new()
        .name("User1")
        .addresses(vec![
            Address::new().city("City1").active(false),
            Address::new().city("City2").active(false),
        ])
        .save_from(&client)
        .await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let updated = User::query()
        .filter(|user| user.name.eq("User1"))
        .array_filter(|user| {
            user.addresses
                .array_filter("address", |address| address.city.eq("City1"))
        })
        .update_one_with_session(&mut session, |user| {
            user.addresses
                .filtered("address", |address| address.active.set(true))
        })
        .await?
        .expect("one user should be updated");

    // The returned document reflects the post-update state and the array
    // filter touched only the matching element.
    assert!(updated.addresses[0].active);
    assert!(!updated.addresses[1].active);

    // A sessionless reader still sees the pre-transaction state.
    let outside = User::find_by_id_from(id, &client)
        .await?
        .expect("document should exist outside the transaction");
    assert!(!outside.addresses[0].active);

    session.abort_transaction().await?;

    let after_abort = User::find_by_id_from(id, &client)
        .await?
        .expect("document should still exist after abort");
    assert!(
        !after_abort.addresses[0].active,
        "aborted typed update should roll back"
    );

    Ok(())
}

// Run test: cargo nextest run typed_update_all_with_session_persists_on_commit
#[tokio::test]
async fn typed_update_all_with_session_persists_on_commit() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("typed_update_all")]
    pub struct Message {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        channel: String,
        archived: bool,
    }

    let client = client().await?;
    Message::clear_from(&client).await?;

    Message::new()
        .channel("news")
        .archived(false)
        .save_from(&client)
        .await?;
    Message::new()
        .channel("news")
        .archived(false)
        .save_from(&client)
        .await?;
    Message::new()
        .channel("chat")
        .archived(false)
        .save_from(&client)
        .await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let result = Message::query()
        .filter(|message| message.channel.eq("news"))
        .update_all_with_session(&mut session, |message| message.archived.set(true))
        .await?;
    assert_eq!(result.modified_count, 2);

    session.commit_transaction().await?;

    assert_eq!(
        Message::count_from(doc! { "archived": true }, &client).await?,
        2,
        "committed bulk update should persist"
    );

    Ok(())
}

// Run test: cargo nextest run typed_delete_one_with_session_respects_sort_and_rolls_back
#[tokio::test]
async fn typed_delete_one_with_session_respects_sort_and_rolls_back() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("typed_delete_one")]
    pub struct Job {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        attempts: i32,
    }

    let client = client().await?;
    Job::clear_from(&client).await?;

    Job::new()
        .name("old")
        .attempts(9)
        .save_from(&client)
        .await?;
    Job::new()
        .name("new")
        .attempts(1)
        .save_from(&client)
        .await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let deleted = Job::query()
        .sort_by(|job| job.attempts.desc())
        .delete_one_with_session(&mut session)
        .await?
        .expect("one job should be deleted");
    assert_eq!(deleted.name, "old", "sorting should select the deleted job");

    // The session no longer sees the deleted document.
    assert_eq!(Job::query().count_with_session(&mut session).await?, 1);

    session.abort_transaction().await?;

    assert_eq!(
        Job::count_from(doc! {}, &client).await?,
        2,
        "aborted typed delete should roll back"
    );

    Ok(())
}

// Run test: cargo nextest run typed_delete_all_with_session_persists_on_commit
#[tokio::test]
async fn typed_delete_all_with_session_persists_on_commit() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("typed_delete_all")]
    pub struct Draft {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        stale: bool,
    }

    let client = client().await?;
    Draft::clear_from(&client).await?;

    Draft::new().stale(true).save_from(&client).await?;
    Draft::new().stale(true).save_from(&client).await?;
    Draft::new().stale(false).save_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let result = Draft::query()
        .filter(|draft| draft.stale.eq(true))
        .delete_all_with_session(&mut session)
        .await?;
    assert_eq!(result.deleted_count, 2);

    session.commit_transaction().await?;

    assert_eq!(
        Draft::count_from(doc! {}, &client).await?,
        1,
        "committed bulk delete should persist"
    );

    Ok(())
}

// Run test: cargo nextest run typed_session_bulk_writes_reject_query_modifiers
#[tokio::test]
async fn typed_session_bulk_writes_reject_query_modifiers() -> TestResult {
    use oximod::{BulkWriteOperation, OxiModError, QueryError, QueryModifier};

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("typed_session_bulk_preflight")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        active: bool,
    }

    let client = client().await?;
    let mut session = client.start_session().await?;

    let delete_sort_error = User::query()
        .sort_by(|user| user.name.asc())
        .delete_all_with_session(&mut session)
        .await
        .expect_err("delete_all_with_session should reject sorting");

    assert!(matches!(
        delete_sort_error,
        OxiModError::Query(QueryError::UnsupportedBulkWriteModifier {
            operation: BulkWriteOperation::DeleteAll,
            modifier: QueryModifier::Sort,
        })
    ));

    let update_limit_error = User::query()
        .limit(1)
        .update_all_with_session(&mut session, |user| user.active.set(true))
        .await
        .expect_err("update_all_with_session should reject limits");

    assert!(matches!(
        update_limit_error,
        OxiModError::Query(QueryError::UnsupportedBulkWriteModifier {
            operation: BulkWriteOperation::UpdateAll,
            modifier: QueryModifier::Limit,
        })
    ));

    Ok(())
}

// Run test: cargo nextest run save_with_session_rejects_invalid_model_before_insert
#[tokio::test]
async fn save_with_session_rejects_invalid_model_before_insert() -> TestResult {
    use oximod::OxiModError;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("validation_guard")]
    pub struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min = 1)]
        pages: i32,
    }

    let client = client().await?;
    Book::clear_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let error = Book::new()
        .pages(0)
        .save_with_session(&mut session)
        .await
        .expect_err("an invalid model must not be inserted");
    assert!(
        matches!(error, OxiModError::Validation { .. }),
        "expected a validation error, got: {error:?}"
    );

    // The rejected document is not visible inside the transaction...
    assert_eq!(Book::count_with_session(doc! {}, &mut session).await?, 0);

    session.commit_transaction().await?;

    // ...and not outside it either.
    assert_eq!(Book::count_from(doc! {}, &client).await?, 0);

    Ok(())
}

// Run test: cargo nextest run session_operations_preserve_hook_firing_order
static SESSION_HOOK_EVENTS: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());

fn record_hook_event(event: &'static str) {
    SESSION_HOOK_EVENTS
        .lock()
        .expect("hook event lock should not be poisoned")
        .push(event);
}

#[tokio::test]
async fn session_operations_preserve_hook_firing_order() -> TestResult {
    use oximod::Hooks;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("session_hooks")]
    #[hooks]
    pub struct Audit {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        message: String,
    }

    #[async_trait::async_trait]
    impl Hooks for Audit {
        async fn pre_save(&self) -> Result<(), oximod::OxiModError> {
            record_hook_event("pre_save");
            Ok(())
        }

        async fn post_save(&self) -> Result<(), oximod::OxiModError> {
            record_hook_event("post_save");
            Ok(())
        }

        async fn pre_save_mut(&mut self) -> Result<(), oximod::OxiModError> {
            record_hook_event("pre_save_mut");
            Ok(())
        }

        async fn post_save_mut(&mut self) -> Result<(), oximod::OxiModError> {
            record_hook_event("post_save_mut");
            Ok(())
        }

        async fn pre_find(_id: ObjectId) -> Result<(), oximod::OxiModError> {
            record_hook_event("pre_find");
            Ok(())
        }

        async fn post_find(_found: &Option<Self>) -> Result<(), oximod::OxiModError> {
            record_hook_event("post_find");
            Ok(())
        }

        async fn pre_update(
            _id: ObjectId,
            _update: &mongodb::bson::Document,
        ) -> Result<(), oximod::OxiModError> {
            record_hook_event("pre_update");
            Ok(())
        }

        async fn post_update(
            _id: ObjectId,
            _update: &mongodb::bson::Document,
        ) -> Result<(), oximod::OxiModError> {
            record_hook_event("post_update");
            Ok(())
        }

        async fn pre_delete(_id: ObjectId) -> Result<(), oximod::OxiModError> {
            record_hook_event("pre_delete");
            Ok(())
        }

        async fn post_delete(_id: ObjectId) -> Result<(), oximod::OxiModError> {
            record_hook_event("post_delete");
            Ok(())
        }
    }

    let client = client().await?;
    Audit::clear_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let id = Audit::new()
        .message("created")
        .save_with_session(&mut session)
        .await?;
    Audit::new()
        .message("mutated")
        .save_mut_with_session(&mut session)
        .await?;
    Audit::find_by_id_with_session(id, &mut session).await?;
    Audit::update_by_id_with_session(id, doc! { "$set": { "message": "updated" } }, &mut session)
        .await?;
    Audit::delete_by_id_with_session(id, &mut session).await?;

    session.abort_transaction().await?;

    let events = SESSION_HOOK_EVENTS
        .lock()
        .expect("hook event lock should not be poisoned")
        .clone();
    assert_eq!(
        events,
        vec![
            "pre_save",
            "post_save",
            "pre_save_mut",
            "post_save_mut",
            "pre_find",
            "post_find",
            "pre_update",
            "post_update",
            "pre_delete",
            "post_delete",
        ],
        "session-aware helpers should fire the same hooks in the same order"
    );

    Ok(())
}

// Run test: cargo nextest run session_duplicate_key_classifies_as_database_error
#[tokio::test]
async fn session_duplicate_key_classifies_as_database_error() -> TestResult {
    use mongodb::error::{ErrorKind, WriteFailure};
    use oximod::OxiModError;
    use std::error::Error as StdError;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("session_duplicate_key")]
    pub struct Account {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique)]
        email: String,
    }

    let client = client().await?;
    Account::clear_from(&client).await?;

    // The documented startup pattern: establish indexes before transactional
    // work begins.
    Account::init_indexes_from(&client).await?;

    Account::new()
        .email("first@example.com")
        .save_from(&client)
        .await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let error = Account::new()
        .email("first@example.com")
        .save_with_session(&mut session)
        .await
        .expect_err("the unique index should reject the duplicate inside the transaction");

    assert!(
        matches!(error, OxiModError::Database { .. }),
        "expected a duplicate key through save_with_session to classify as Database, got: {error:?}"
    );

    let driver_error = error
        .source()
        .and_then(|source| source.downcast_ref::<mongodb::error::Error>())
        .expect("the original driver error should be preserved through source()");

    let code = match &*driver_error.kind {
        ErrorKind::Write(WriteFailure::WriteError(write_error)) => Some(write_error.code),
        ErrorKind::Command(command_error) => Some(command_error.code),
        _ => None,
    };
    assert_eq!(code, Some(11000), "expected duplicate key code 11000");

    session.abort_transaction().await?;

    Ok(())
}

// Run test: cargo nextest run save_with_session_does_not_establish_indexes
#[tokio::test]
async fn save_with_session_does_not_establish_indexes() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("session_tx_test")]
    #[collection("session_no_index_creation")]
    pub struct Ticket {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique)]
        code: String,
    }

    let client = client().await?;
    Ticket::clear_from(&client).await?;

    // Intentionally no init_indexes_from: the session-aware save must not
    // establish the declared unique index as hidden out-of-transaction work.
    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    Ticket::new()
        .code("T-1")
        .save_with_session(&mut session)
        .await?;
    Ticket::new()
        .code("T-1")
        .save_with_session(&mut session)
        .await?;

    session.commit_transaction().await?;

    // Both duplicates committed, so no unique index can have been created.
    assert_eq!(
        Ticket::count_from(doc! { "code": "T-1" }, &client).await?,
        2
    );

    let index_names = Ticket::get_collection_from(&client)?
        .list_index_names()
        .await?;
    assert!(
        index_names.iter().all(|name| name == "_id_"),
        "save_with_session must not create declared indexes, found: {index_names:?}"
    );

    Ok(())
}
