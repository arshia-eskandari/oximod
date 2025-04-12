use mongodb::bson::{doc, oid::ObjectId};
use monoxide_core::feature::conn::client::set_global_client;
use monoxide_macros::Model;
use testresult::TestResult;
use monoxide_core::feature::model::Model;
use serde::{Deserialize, Serialize};

// Run test: cargo nextest run deletes_document_by_id_correctly
#[tokio::test]
async fn deletes_document_by_id_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("db_name")]
    #[collection("collection_name")]
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
        age: 40,
        active: true,
    };

    let id = user._id.unwrap();
    user.save().await?;

    let deleted = User::delete_by_id(id).await?;
    assert!(deleted);

    let result = User::find_by_id(id).await?;
    assert!(result.is_none());

    Ok(())
}
