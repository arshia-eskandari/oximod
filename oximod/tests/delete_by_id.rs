use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run deletes_document_by_id_correctly
#[tokio::test]
async fn deletes_document_by_id_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_by_id_test_deletes_document_by_id_correctly")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let id = ObjectId::new();
    let user = User::default().id(id).name("User1").age(40).active(true);

    user.save().await?;

    let collection = User::get_collection()?;

    let deleted = collection.delete_one(doc! { "_id": id }).await?;
    assert_eq!(deleted.deleted_count, 1);

    let result = collection.find_one(doc! { "_id": id }).await?;
    assert!(result.is_none());

    Ok(())
}

// Run test: cargo nextest run delete_by_id_no_matching_document
#[tokio::test]
async fn delete_by_id_no_matching_document() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_by_id_test_no_matching_document")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let id = ObjectId::new(); // but never inserted
    let collection = User::get_collection()?;

    let deleted = collection.delete_one(doc! { "_id": id }).await?;
    assert_eq!(
        deleted.deleted_count, 0,
        "No documents should have been deleted for non-existent ID"
    );

    Ok(())
}

// Run test: cargo nextest run deletes_document_by_id_correctly_with_email
#[tokio::test]
async fn deletes_document_by_id_correctly_with_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_by_id_test_deletes_document_by_id_correctly_with_email")]
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

    let id = ObjectId::new();
    let user = User::default()
        .id(id)
        .name("User1")
        .age(40)
        .active(true)
        .email("user1@example.com");

    user.save().await?;

    let collection = User::get_collection()?;

    let deleted = collection.delete_one(doc! { "_id": id }).await?;
    assert_eq!(deleted.deleted_count, 1);

    let result = collection.find_one(doc! { "_id": id }).await?;
    assert!(result.is_none());

    Ok(())
}

// Run test: cargo nextest run delete_by_id_no_matching_document_with_email
#[tokio::test]
async fn delete_by_id_no_matching_document_with_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_by_id_test_no_matching_document_with_email")]
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

    let id = ObjectId::new(); // not inserted
    let collection = User::get_collection()?;

    let deleted = collection.delete_one(doc! { "_id": id }).await?;
    assert_eq!(
        deleted.deleted_count, 0,
        "No documents should have been deleted for non-existent ID"
    );

    Ok(())
}
