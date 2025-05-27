use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run counts_matching_documents_correctly
#[tokio::test]
async fn counts_matching_documents_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("count")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1".to_string()).age(30).active(true),
        User::default().name("User3".to_string()).age(30).active(false),
        User::default().name("User3".to_string()).age(25).active(true)
    ];

    for user in users {
        user.save().await?;
    }

    let count = User::count(doc! { "age": 30 }).await?;
    assert_eq!(count, 2);

    Ok(())
}
