use mongodb::bson::{doc, oid::ObjectId};
use monoxide_core::feature::conn::client::set_global_client;
use monoxide_macros::Model;
use testresult::TestResult;
use monoxide_core::feature::model::Model;
use serde::{Deserialize, Serialize};

// Run test: cargo nextest run finds_document_by_id_correctly
#[tokio::test]
async fn finds_document_by_id_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_by_id")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    let user = User {
        _id: Some(ObjectId::new()),
        name: "User1".to_string(),
        age: 33,
        active: true,
    };

    let id = user._id.unwrap();
    user.save().await?;

    let found = User::find_by_id(id).await?;
    assert!(found.is_some());

    if let Some(user) = found {
        assert_eq!(user._id, Some(id));
        assert_eq!(user.name, "User1");
    }

    Ok(())
}
