mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{ Deserialize, Serialize };
use testresult::TestResult;

/// A simple model used to test the `new`, `default`, and builder APIs.
#[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
#[db("test")]
#[collection("builder_tests")]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,
    name: String,
    age: i32,
    active: bool,
}

// Run test: cargo nextest run new_and_default_are_equivalent
#[tokio::test]
async fn new_and_default_are_equivalent() -> TestResult {
    let user_new = User::new();
    let user_default = User::default();

    assert_eq!(user_new, user_default);
    Ok(())
}

// Run test: cargo nextest run builder_sets_all_fields
#[tokio::test]
async fn builder_sets_all_fields() -> TestResult {
    let id = ObjectId::new();
    let user = User::default().id(id.clone()).name("Alice".to_string()).age(30).active(true);

    assert_eq!(user._id, Some(id));
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
    assert!(user.active);

    Ok(())
}

// Run test: cargo nextest run builder_partial_fields_default_rest
#[tokio::test]
async fn builder_partial_fields_default_rest() -> TestResult {
    let user = User::default().name("Bob".to_string());

    // name should be set, rest should be their respective defaults
    assert_eq!(user.name, "Bob");
    assert_eq!(user.age, 0);
    assert_eq!(user.active, false);
    assert_eq!(user._id, None);

    Ok(())
}

// Run test: cargo nextest run builder_and_save_works_end_to_end
#[tokio::test]
async fn builder_and_save_works_end_to_end() -> TestResult {
    init().await;
    User::clear().await?;

    let saved_id = User::default().name("Charlie".to_string()).age(42).active(true).save().await?;

    assert_ne!(saved_id, ObjectId::default());

    let fetched = User::find_by_id(saved_id).await?;
    assert!(fetched.is_some());

    let user = fetched.unwrap();
    assert_eq!(user.name, "Charlie");
    assert_eq!(user.age, 42);
    assert!(user.active);

    Ok(())
}
