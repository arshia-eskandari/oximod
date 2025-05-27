use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run deletes_first_matching_document_only
#[tokio::test]
async fn deletes_first_matching_document_only() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete_one")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1".to_string()).age(50).active(false),
        User::default().name("User2".to_string()).age(50).active(false)
    ];

    for user in users {
        user.save().await?;
    }

    let deleted = User::delete_one(doc! { "age": 50, "active": false }).await?;
    assert_eq!(deleted.deleted_count, 1);

    Ok(())
}
