use mongodb::{ bson::{ doc, oid::ObjectId }, options::IndexOptions, IndexModel };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run uses_get_collection_and_manual_indexing
#[tokio::test]
async fn uses_get_collection_and_manual_indexing() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("manual_index_test")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        username: String,
        email: String,
    }

    User::clear().await?;

    let collection = User::get_collection()?;

    let index_model = IndexModel::builder()
        .keys(doc! { "email": 1 })
        .options(
            IndexOptions::builder()
                .unique(Some(true))
                .name(Some("email_unique_index".to_string()))
                .build()
        )
        .build();

    collection.create_index(index_model).await?;

    let user = User::default().username("User1".to_string()).email("user1@example.com".to_string());

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    let fetched = User::find_one(doc! { "email": "user1@example.com" }).await?;
    assert!(fetched.is_some());

    Ok(())
}
