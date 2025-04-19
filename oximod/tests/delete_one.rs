use mongodb::bson::{doc, oid::ObjectId};
use oximod::{set_global_client, Model};
use testresult::TestResult;
use serde::{Deserialize, Serialize};

// Run test: cargo nextest run deletes_first_matching_document_only
#[tokio::test]
async fn deletes_first_matching_document_only() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

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
        User {
            _id: None,
            name: "User1".to_string(),
            age: 50,
            active: false,
        },
        User {
            _id: None,
            name: "User2".to_string(),
            age: 50,
            active: false,
        },
    ];

    for user in users {
        user.save().await?;
    }

    let deleted = User::delete_one(doc! { "age": 50, "active": false }).await?;
    assert_eq!(deleted.deleted_count, 1);

    Ok(())
}
