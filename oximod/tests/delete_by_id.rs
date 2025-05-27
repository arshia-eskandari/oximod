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
    #[collection("delete_by_id")]
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
