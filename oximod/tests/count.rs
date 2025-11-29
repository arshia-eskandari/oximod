use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run counts_matching_documents_correctly
#[tokio::test]
async fn counts_matching_documents_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("count_test_counts_matching_documents_correctly")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default()
            .name("User1".to_string())
            .age(30)
            .active(true),
        User::default()
            .name("User3".to_string())
            .age(30)
            .active(false),
        User::default()
            .name("User3".to_string())
            .age(25)
            .active(true),
    ];

    for user in users {
        user.save().await?;
    }

    let count = User::count(doc! { "age": 30 }).await?;
    assert_eq!(count, 2);

    Ok(())
}

// Run test: cargo nextest run counts_no_matching_documents
#[tokio::test]
async fn counts_no_matching_documents() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("count_test_counts_no_matching_documents")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let count = User::count(doc! { "age": 999 }).await?;
    assert_eq!(count, 0);

    Ok(())
}

// Run test: cargo nextest run counts_matching_documents_by_email_correctly
#[tokio::test]
async fn counts_matching_documents_by_email_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("count_test_counts_matching_documents_by_email_correctly")]
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
            .name("User1".to_string())
            .age(30)
            .active(true)
            .email("shared@example.com".to_string()),
        User::default()
            .name("User2".to_string())
            .age(30)
            .active(false)
            .email("shared@example.com".to_string()),
        User::default()
            .name("User3".to_string())
            .age(25)
            .active(true)
            .email("unique@example.com".to_string()),
    ];

    for user in users {
        user.save().await?;
    }

    let count = User::count(doc! { "email": "shared@example.com" }).await?;
    assert_eq!(count, 2);

    Ok(())
}

// Run test: cargo nextest run counts_no_matching_documents_by_email
#[tokio::test]
async fn counts_no_matching_documents_by_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("count_test_counts_no_matching_documents_by_email")]
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

    let count = User::count(doc! { "email": "notfound@example.com" }).await?;
    assert_eq!(count, 0);

    Ok(())
}
