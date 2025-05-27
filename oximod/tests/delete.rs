use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run deletes_multiple_matching_documents
#[tokio::test]
async fn deletes_multiple_matching_documents() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("delete")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1".to_string()).age(45).active(false),
        User::default().name("User2".to_string()).age(38).active(false),
        User::default().name("User3".to_string()).age(38).active(true)
    ];

    for user in users {
        user.save().await?;
    }

    let deleted_result = User::delete(doc! { "active": false }).await?;
    assert_eq!(deleted_result.deleted_count, 2);

    Ok(())
}
