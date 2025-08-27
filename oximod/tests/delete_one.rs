use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run deletes_first_matching_document_only
#[tokio::test]
async fn deletes_first_matching_document_only() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_one_test_deletes_first_matching_document_only")]
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
            .age(50)
            .active(false),
        User::default()
            .name("User2".to_string())
            .age(50)
            .active(false),
    ];

    for user in users {
        user.save().await?;
    }

    let deleted = User::delete_one(doc! { "age": 50, "active": false }).await?;
    assert_eq!(deleted.deleted_count, 1);

    Ok(())
}

// Run test: cargo nextest run delete_one_no_matching_document
#[tokio::test]
async fn delete_one_no_matching_document() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_one_test_no_matching_document")]
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
            .age(50)
            .active(true),
        User::default()
            .name("User2".to_string())
            .age(50)
            .active(true),
    ];

    for user in users {
        user.save().await?;
    }

    let deleted = User::delete_one(doc! { "age": 50, "active": false }).await?;
    assert_eq!(
        deleted.deleted_count, 0,
        "No documents should have been deleted"
    );

    Ok(())
}

// Run test: cargo nextest run deletes_first_matching_document_by_email_only
#[tokio::test]
async fn deletes_first_matching_document_by_email_only() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_one_test_deletes_first_matching_document_by_email_only")]
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
            .age(50)
            .active(false)
            .email("shared@example.com".to_string()),
        User::default()
            .name("User2".to_string())
            .age(50)
            .active(false)
            .email("shared@example.com".to_string()),
    ];

    for user in users {
        user.save().await?;
    }

    let deleted = User::delete_one(doc! { "email": "shared@example.com" }).await?;
    assert_eq!(deleted.deleted_count, 1);

    Ok(())
}

// Run test: cargo nextest run delete_one_no_matching_document_by_email
#[tokio::test]
async fn delete_one_no_matching_document_by_email() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_one_test_no_matching_document_by_email")]
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
            .age(50)
            .active(true)
            .email("user1@example.com".to_string()),
        User::default()
            .name("User2".to_string())
            .age(50)
            .active(true)
            .email("user2@example.com".to_string()),
    ];

    for user in users {
        user.save().await?;
    }

    let deleted = User::delete_one(doc! { "email": "notfound@example.com" }).await?;
    assert_eq!(
        deleted.deleted_count, 0,
        "No documents should have been deleted"
    );

    Ok(())
}
