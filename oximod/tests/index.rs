use mongodb::bson::{DateTime, oid::ObjectId};
use oximod::{set_global_client, Model};
use testresult::TestResult;
use serde::{Deserialize, Serialize};

// Run test: cargo nextest run creates_indexes_correctly
#[tokio::test] // Might throw `expected Expr rust-analyzer` so disable "macro-error"
async fn creates_indexes_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_test")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "name_idx")]
        name: String,

        #[index(sparse, order = "-1")]
        age: Option<i32>,

        #[index(expire_after_secs = 3600)]
        created_at: Option<DateTime>,

        active: bool,
    }

    User::clear().await?;

    let user = User {
        _id: None,
        name: "IndexUser".to_string(),
        age: Some(25),
        created_at: Some(DateTime::now()),
        active: true,
    };

    // This will trigger create_indexes() inside save
    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(()) 
}
