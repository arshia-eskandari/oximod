use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run deletes_document_by_id_correctly
#[tokio::test]
async fn deletes_document_by_id_correctly() -> TestResult {
    init().await;

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
    let user = User::default().id(id.clone()).name("User1".to_string()).age(40).active(true);

    user.save().await?;

    let deleted = User::delete_by_id(id).await?;
    assert_eq!(deleted.deleted_count, 1);

    let result = User::find_by_id(id).await?;
    assert!(result.is_none());

    Ok(())
}

// Run test: cargo nextest run delete_by_id_no_matching_document
#[tokio::test]
async fn delete_by_id_no_matching_document() -> TestResult {
    init().await;

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

    let deleted = User::delete_by_id(id).await?;
    assert_eq!(
        deleted.deleted_count,
        0,
        "No documents should have been deleted for non-existent ID"
    );

    Ok(())
}
