//! Integration tests for deleting multiple documents through raw collection access.
//!
//! The tests cover matching and nonmatching filters against ordinary fields
//! and optional email fields, including the resulting deleted and remaining counts.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run deletes_multiple_matching_documents
#[tokio::test]
async fn deletes_multiple_matching_documents() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_test_deletes_multiple_matching_documents")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1").age(45).active(false),
        User::default().name("User2").age(38).active(false),
        User::default().name("User3").age(38).active(true),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;

    let deleted_result = collection.delete_many(doc! { "active": false }).await?;
    assert_eq!(deleted_result.deleted_count, 2);

    let remaining = collection.count_documents(doc! {}).await?;
    assert_eq!(remaining, 1);

    Ok(())
}

// Run test: cargo nextest run delete_no_matching_documents
#[tokio::test]
async fn delete_no_matching_documents() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_test_no_matching_documents")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1").age(45).active(true),
        User::default().name("User2").age(38).active(true),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;

    let deleted_result = collection.delete_many(doc! { "active": false }).await?;
    assert_eq!(
        deleted_result.deleted_count, 0,
        "No documents should have been deleted"
    );

    let remaining = collection.count_documents(doc! {}).await?;
    assert_eq!(remaining, 2);

    Ok(())
}

// Run test: cargo nextest run deletes_multiple_matching_documents_by_email
#[tokio::test]
async fn deletes_multiple_matching_documents_by_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_test_deletes_multiple_matching_documents_by_email")]
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

    let users = vec![
        User::default()
            .name("User1")
            .age(45)
            .active(false)
            .email("shared@example.com"),
        User::default()
            .name("User2")
            .age(38)
            .active(false)
            .email("shared@example.com"),
        User::default()
            .name("User3")
            .age(38)
            .active(true)
            .email("user3@example.com"),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;

    let deleted_result = collection
        .delete_many(doc! { "email": "shared@example.com" })
        .await?;
    assert_eq!(deleted_result.deleted_count, 2);

    let remaining = collection.count_documents(doc! {}).await?;
    assert_eq!(remaining, 1);

    Ok(())
}

// Run test: cargo nextest run delete_no_matching_documents_by_email
#[tokio::test]
async fn delete_no_matching_documents_by_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_test_no_matching_documents_by_email")]
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

    let users = vec![
        User::default()
            .name("User1")
            .age(45)
            .active(true)
            .email("user1@example.com"),
        User::default()
            .name("User2")
            .age(38)
            .active(true)
            .email("user2@example.com"),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;

    let deleted_result = collection
        .delete_many(doc! { "email": "notfound@example.com" })
        .await?;
    assert_eq!(
        deleted_result.deleted_count, 0,
        "No documents should have been deleted"
    );

    let remaining = collection.count_documents(doc! {}).await?;
    assert_eq!(remaining, 2);

    Ok(())
}
