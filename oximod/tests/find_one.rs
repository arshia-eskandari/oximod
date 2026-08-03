//! Integration tests for finding one document through raw collection access.
//!
//! The tests cover successful and empty lookups using ordinary field filters
//! and optional email-field filters.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run finds_first_matching_document_correctly
#[tokio::test]
async fn finds_first_matching_document_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_one_test_finds_first_matching_document_correctly")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1").age(22).active(true),
        User::default().name("User2").age(22).active(false),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;
    let matched = collection.find_one(doc! { "age": 22 }).await?;
    assert!(matched.is_some());

    if let Some(user) = matched {
        assert_eq!(user.age, 22);
        assert!(["User1", "User2"].contains(&user.name.as_str()));
    }

    Ok(())
}

// Run test: cargo nextest run finds_first_matching_document_none_when_no_match
#[tokio::test]
async fn finds_first_matching_document_none_when_no_match() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_one_test_none_when_no_match")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1").age(22).active(true),
        User::default().name("User2").age(22).active(false),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;
    let matched = collection.find_one(doc! { "age": 99 }).await?;
    assert!(matched.is_none(), "Expected no document to match");

    Ok(())
}

// Run test: cargo nextest run finds_first_matching_document_by_email
#[tokio::test]
async fn finds_first_matching_document_by_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_one_test_by_email")]
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
            .age(22)
            .active(true)
            .email("user1@example.com"),
        User::default()
            .name("User2")
            .age(25)
            .active(false)
            .email("user2@example.com"),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;
    let matched = collection
        .find_one(doc! { "email": "user1@example.com" })
        .await?;
    assert!(matched.is_some(), "Expected to find user by email");

    if let Some(user) = matched {
        assert_eq!(user.email.as_deref(), Some("user1@example.com"));
        assert_eq!(user.name, "User1");
    }

    Ok(())
}

// Run test: cargo nextest run finds_first_matching_document_by_email_none_when_no_match
#[tokio::test]
async fn finds_first_matching_document_by_email_none_when_no_match() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_one_test_by_email_none_when_no_match")]
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
            .age(22)
            .active(true)
            .email("user1@example.com"),
        User::default()
            .name("User2")
            .age(25)
            .active(false)
            .email("user2@example.com"),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;
    let matched = collection
        .find_one(doc! { "email": "notfound@example.com" })
        .await?;
    assert!(
        matched.is_none(),
        "Expected no user to match non-existing email"
    );

    Ok(())
}
