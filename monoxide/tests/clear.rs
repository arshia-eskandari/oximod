use mongodb::bson::{doc, oid::ObjectId};
use monoxide_core::feature::conn::client::set_global_client;
use monoxide_macros::Model;
use testresult::TestResult;
use monoxide_core::feature::model::Model;
use serde::{Deserialize, Serialize};

// Run test: cargo nextest run clears_collection_successfully
#[tokio::test]
async fn clears_collection_successfully() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

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
        User {
            _id: None,
            name: "User1".to_string(),
            age: 22,
            active: true,
        },
        User {
            _id: None,
            name: "User2".to_string(),
            age: 28,
            active: false,
        },
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
