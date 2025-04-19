use mongodb::bson::oid::ObjectId;
use oximod::{set_global_client, Model};
use testresult::TestResult;
use serde::{Deserialize, Serialize};

// Run test: cargo nextest run saves_document_without_id_correctly
#[tokio::test]
async fn saves_document_without_id_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize)]
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
        _id: None,
        name: "User1".to_string(),
        age: 25,
        active: true,
    };

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());    

    Ok(())
}

// Run test: cargo nextest run saves_document_with_id_correctly
#[tokio::test]
async fn saves_document_with_id_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("save")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let user = User {
        _id: Some(ObjectId::new()),
        name: "User1".to_string(),
        age: 30,
        active: false,
    };

    let result = user.save().await?;
    assert_eq!(result, user._id.unwrap());

    Ok(())
}
