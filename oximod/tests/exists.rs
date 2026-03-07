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
