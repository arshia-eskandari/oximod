use mongodb::bson::{ DateTime, oid::ObjectId, doc };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };
use std::{ thread::sleep, time::Duration };

mod common;
use common::init;

// Run test: cargo nextest run creates_indexes_correctly
#[tokio::test] // Might throw `expected Expr rust-analyzer` so disable "macro-error"
async fn creates_indexes_correctly() -> TestResult {
    init().await;

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

    let user = User::default()
        .name("IndexUser".to_string())
        .age(25)
        .created_at(DateTime::now())
        .active(true);

    // This will trigger create_indexes() inside save
    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run ttl_index_removes_expired_documents
#[tokio::test]
async fn ttl_index_removes_expired_documents() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ttl_test")]
    pub struct Session {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(expire_after_secs = 2)]
        created_at: Option<DateTime>,
    }

    Session::clear().await?;

    // Insert a document with a created_at timestamp in the past
    let expired_session = Session::default().created_at(
        DateTime::from_millis(DateTime::now().timestamp_millis() - 10_000)
    );

    expired_session.save().await?;

    // Give MongoDB TTL monitor enough time to delete the expired document
    sleep(Duration::from_secs(65));

    let remaining = Session::find(doc! {}).await?;
    assert_eq!(remaining.len(), 0, "Expected document to be expired and deleted");

    Ok(())
}
