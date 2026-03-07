use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run clears_collection_successfully
#[tokio::test]
async fn clears_collection_successfully() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("clear_test_clears_collection_successfully")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    let users = vec![
        User::default().name("User1").age(22).active(true),
        User::default().name("User2").age(28).active(false),
    ];

    for user in users {
        user.save().await?;
    }

    let count_before = User::get_collection()?.count_documents(doc! {}).await?;
    assert!(count_before >= 2);

    let result = User::clear().await?;
    assert!(result.deleted_count >= 2);

    let count_after = User::get_collection()?.count_documents(doc! {}).await?;
    assert_eq!(count_after, 0);

    Ok(())
}
