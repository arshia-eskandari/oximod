use mongodb::bson::{doc, oid::ObjectId};
use monoxide_core::feature::conn::client::set_global_client;
use monoxide_macros::Model;
use testresult::TestResult;
use monoxide_core::feature::model::Model;
use serde::{Deserialize, Serialize};

// Run test: cargo nextest run finds_first_matching_document_correctly
#[tokio::test]
async fn finds_first_matching_document_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_one")]
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
            age: 22,
            active: true,
        },
        User {
            _id: None,
            name: "User2".to_string(),
            age: 22,
            active: false,
        },
    ];

    for user in users {
        user.save().await?;
    }

    let matched = User::find_one(doc! { "age": 22 }).await?;
    assert!(matched.is_some());

    if let Some(user) = matched {
        assert_eq!(user.age, 22);
        assert!(["User1", "User2"].contains(&user.name.as_str()));
    }

    Ok(())
}
