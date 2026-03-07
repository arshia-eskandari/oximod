use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run finds_multiple_matching_documents
#[tokio::test]
async fn finds_multiple_matching_documents() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_test_finds_multiple_matching_documents")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1").age(28).active(true),
        User::default().name("User2").age(28).active(true),
        User::default().name("User3").age(35).active(true),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;
    let cursor = collection.find(doc! { "age": 28 }).await?;
    let matched_users: Vec<User> = cursor.try_collect().await?;

    assert_eq!(matched_users.len(), 2);

    let names: Vec<String> = matched_users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"User1".to_string()));
    assert!(names.contains(&"User2".to_string()));

    Ok(())
}

// Run test: cargo nextest run finds_no_matching_documents
#[tokio::test]
async fn finds_no_matching_documents() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_test_finds_no_matching_documents")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1").age(28).active(true),
        User::default().name("User2").age(28).active(true),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;
    let cursor = collection.find(doc! { "age": 99 }).await?;
    let matched_users: Vec<User> = cursor.try_collect().await?;

    assert_eq!(matched_users.len(), 0, "Expected no documents to match");

    Ok(())
}

// Run test: cargo nextest run finds_multiple_matching_documents_by_email
#[tokio::test]
async fn finds_multiple_matching_documents_by_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_test_finds_multiple_matching_documents_by_email")]
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
            .age(28)
            .active(true)
            .email("shared@example.com"),
        User::default()
            .name("User2")
            .age(28)
            .active(true)
            .email("shared@example.com"),
        User::default()
            .name("User3")
            .age(35)
            .active(true)
            .email("user3@example.com"),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;
    let cursor = collection
        .find(doc! { "email": "shared@example.com" })
        .await?;
    let matched_users: Vec<User> = cursor.try_collect().await?;

    assert_eq!(matched_users.len(), 2);

    let names: Vec<String> = matched_users.into_iter().map(|u| u.name).collect();
    assert!(names.contains(&"User1".to_string()));
    assert!(names.contains(&"User2".to_string()));

    Ok(())
}

// Run test: cargo nextest run finds_no_matching_documents_by_email
#[tokio::test]
async fn finds_no_matching_documents_by_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_test_finds_no_matching_documents_by_email")]
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
            .age(28)
            .active(true)
            .email("user1@example.com"),
        User::default()
            .name("User2")
            .age(28)
            .active(true)
            .email("user2@example.com"),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;
    let cursor = collection
        .find(doc! { "email": "notfound@example.com" })
        .await?;
    let matched_users: Vec<User> = cursor.try_collect().await?;

    assert_eq!(matched_users.len(), 0, "Expected no documents to match");

    Ok(())
}
