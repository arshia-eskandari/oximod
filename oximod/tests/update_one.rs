use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run updates_first_matching_document_only
#[tokio::test]
async fn updates_first_matching_document_only() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update_one")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1").age(65).active(true),
        User::default().name("User2").age(65).active(true),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;

    let result = collection
        .update_one(doc! { "age": 65 }, doc! { "$set": { "active": false } })
        .await?;

    assert_eq!(result.matched_count, 1);
    assert_eq!(result.modified_count, 1);

    let active_count = collection
        .count_documents(doc! { "age": 65, "active": true })
        .await?;
    let inactive_count = collection
        .count_documents(doc! { "age": 65, "active": false })
        .await?;

    assert_eq!(active_count, 1);
    assert_eq!(inactive_count, 1);

    Ok(())
}

// Run test: cargo nextest run updates_first_matching_document_invalid_update_fails
#[tokio::test]
async fn updates_first_matching_document_invalid_update_fails() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update_one_invalid")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1").age(65).active(true),
        User::default().name("User2").age(65).active(true),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;

    let result = collection
        .update_one(doc! { "age": 65 }, doc! { "$set": "invalid" })
        .await;

    assert!(result.is_err());
    Ok(())
}

// Run test: cargo nextest run updates_one_optional_email_to_valid
#[tokio::test]
async fn updates_one_optional_email_to_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update_one_optional_email_valid")]
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
        User::default().name("User1").age(65).active(true),
        User::default().name("User2").age(65).active(true),
    ];

    for user in users {
        user.save().await?;
    }

    let collection = User::get_collection()?;

    let result = collection
        .update_one(
            doc! { "age": 65 },
            doc! { "$set": { "email": "user@example.com" } },
        )
        .await?;

    assert_eq!(result.matched_count, 1);
    assert_eq!(result.modified_count, 1);

    let updated_count = collection
        .count_documents(doc! { "age": 65, "email": "user@example.com" })
        .await?;
    assert_eq!(updated_count, 1);

    Ok(())
}
