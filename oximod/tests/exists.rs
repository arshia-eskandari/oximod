//! Integration tests for checking whether matching documents exist.
//!
//! The tests cover direct collection lookups and `Model::exists()` with
//! matching and nonmatching filters against ordinary and optional email fields.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run checks_existence_of_matching_document
#[tokio::test]
async fn checks_existence_of_matching_document() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("exists_test_checks_existence_of_matching_document")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let user = User::default().name("User1").age(27).active(true);
    user.save().await?;

    let collection = User::get_collection()?;

    let exists = collection
        .find_one(doc! { "name": "User1" })
        .await?
        .is_some();
    assert!(exists);

    let not_exists = collection
        .find_one(doc! { "name": "SomeoneWhoDoesNotExist" })
        .await?
        .is_some();
    assert!(!not_exists);

    Ok(())
}

// Run test: cargo nextest run checks_existence_of_matching_document_by_email
#[tokio::test]
async fn checks_existence_of_matching_document_by_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("exists_test_checks_existence_of_matching_document_by_email")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        age: i32,
        active: bool,

        #[validate(email)]
        email: Option<String>,
    }

    User::clear().await?;

    let user = User::default()
        .name("User1")
        .age(27)
        .active(true)
        .email("user1@example.com");

    user.save().await?;

    let collection = User::get_collection()?;

    let exists = collection
        .find_one(doc! { "email": "user1@example.com" })
        .await?
        .is_some();
    assert!(exists);

    let not_exists = collection
        .find_one(doc! { "email": "nonexistent@example.com" })
        .await?
        .is_some();
    assert!(!not_exists);

    Ok(())
}

// Run test: cargo nextest run checks_existence_of_matching_document_using_model_helper
#[tokio::test]
async fn checks_existence_of_matching_document_using_model_helper() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("exists_test_checks_existence_of_matching_document_using_model_helper")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let user = User::default().name("User1").age(27).active(true);
    user.save().await?;

    let exists = User::exists(doc! { "name": "User1" }).await?;
    assert!(exists);

    let not_exists = User::exists(doc! { "name": "SomeoneWhoDoesNotExist" }).await?;
    assert!(!not_exists);

    Ok(())
}

// Run test: cargo nextest run exists_is_a_document_level_probe_and_agrees_with_count
#[tokio::test]
async fn exists_is_a_document_level_probe_and_agrees_with_count() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("exists_test_exists_is_a_document_level_probe_and_agrees_with_count")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    // Raw-insert a document that matches the filter but cannot deserialize as
    // `User` (`age` is a string, `active` is missing).
    let raw_collection = User::get_document_collection()?;
    raw_collection
        .insert_one(doc! { "name": "Poison", "age": "twenty-seven" })
        .await?;

    let filter = doc! { "name": "Poison" };

    // The fixture is genuinely undeserializable through the typed collection.
    let typed_read = User::get_collection()?.find_one(filter.clone()).await;
    assert!(
        typed_read.is_err(),
        "raw fixture unexpectedly deserialized as User"
    );

    let exists = User::exists(filter.clone()).await?;
    assert!(exists);

    let count = User::count(filter).await?;
    assert!(count > 0);
    assert_eq!(exists, count > 0);

    let not_exists = User::exists(doc! { "name": "SomeoneWhoDoesNotExist" }).await?;
    assert!(!not_exists);

    Ok(())
}

// Run test: cargo nextest run checks_existence_of_matching_document_by_email_using_model_helper
#[tokio::test]
async fn checks_existence_of_matching_document_by_email_using_model_helper() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("exists_test_checks_existence_of_matching_document_by_email_using_model_helper")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        age: i32,
        active: bool,

        #[validate(email)]
        email: Option<String>,
    }

    User::clear().await?;

    let user = User::default()
        .name("User1")
        .age(27)
        .active(true)
        .email("user1@example.com");

    user.save().await?;

    let exists = User::exists(doc! { "email": "user1@example.com" }).await?;
    assert!(exists);

    let not_exists = User::exists(doc! { "email": "nonexistent@example.com" }).await?;
    assert!(!not_exists);

    Ok(())
}
