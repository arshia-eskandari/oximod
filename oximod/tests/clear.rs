use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run clears_collection_successfully
#[tokio::test]
async fn clears_collection_successfully() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("clear")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    let users = vec![
        User::default().name("User1".to_string()).age(22).active(true),
        User::default().name("User2".to_string()).age(28).active(false)
    ];

    for user in users {
        user.save().await?;
    }

    let count_before = User::count(doc! {}).await?;
    assert!(count_before >= 2);

    let result = User::clear().await?;
    assert!(result.deleted_count >= 2);

    Ok(())
}
