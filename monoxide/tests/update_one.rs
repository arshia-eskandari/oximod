use mongodb::bson::{doc, oid::ObjectId};
use monoxide_core::feature::conn::client::set_global_client;
use monoxide_macros::Model;
use testresult::TestResult;
use monoxide_core::feature::model::Model;
use serde::{Deserialize, Serialize};

// Run test: cargo nextest run updates_first_matching_document_only
#[tokio::test]
async fn updates_first_matching_document_only() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update_one")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User {
            _id: None,
            name: "User1".to_string(),
            age: 65,
            active: true,
        },
        User {
            _id: None,
            name: "User2".to_string(),
            age: 65,
            active: true,
        },
    ];

    for user in users {
        user.save().await?;
    }

    // Only one of the users with age 65 should be updated
    let result = User::update_one(
        doc! { "age": 65 },
        doc! { "$set": { "active": false } },
    )
    .await?;

    assert_eq!(result.matched_count, 1);
    assert_eq!(result.modified_count, 1);

    Ok(())
}
