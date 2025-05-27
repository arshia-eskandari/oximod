use mongodb::bson::oid::ObjectId;
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run saves_document_without_id_correctly
#[tokio::test]
async fn saves_document_without_id_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("db_name")]
    #[collection("collection_name")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let user = User::default().name("User1".to_string()).age(25).active(true);

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run saves_document_with_id_correctly
#[tokio::test]
async fn saves_document_with_id_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("save")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let id = ObjectId::new();
    let user = User::default().id(id.clone()).name("User1".to_string()).age(30).active(false);

    let result = user.save().await?;
    assert_eq!(result, id);

    Ok(())
}
