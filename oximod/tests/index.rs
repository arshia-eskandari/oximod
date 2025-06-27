use mongodb::{
    bson::{ doc, oid::ObjectId, DateTime },
    options::{ IndexVersion, TextIndexVersion },
};
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };
use std::{ thread::sleep, time::Duration };
use futures_util::TryStreamExt;

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

// Run test: cargo nextest run index_version_is_applied_correctly
#[tokio::test]
async fn index_version_is_applied_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("version_index_test")]
    pub struct VersionedIndex {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(version = 2, name = "v2_idx")]
        data: String,
    }

    VersionedIndex::clear().await?;

    let item = VersionedIndex::default().data("hello".to_string());
    item.save().await?;

    // Confirm the index is created with version 2
    let mut cursor = VersionedIndex::get_collection()
        .expect("Failed to get collection")
        .list_indexes().await?;

    let mut found = false;

    while let Some(index) = cursor.try_next().await? {
        if let Some(opts) = index.options {
            if opts.name.as_deref() == Some("v2_idx") {
                if let Some(IndexVersion::V2) = opts.version {
                    found = true;
                }
            }
        }
    }

    assert!(found, "Expected index with name 'v2_idx'");

    Ok(())
}

// Run test: cargo nextest run text_index_version_is_applied_correctly
#[tokio::test]
async fn text_index_version_is_applied_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("text_index_version_is_applied_correctly")]
    pub struct TestModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(text_index_version = 2, name = "text_v2_idx")]
        data: String,
    }

    TestModel::clear().await?;

    let item = TestModel::default().data("hello".to_string());
    item.save().await?;

    let mut cursor = TestModel::get_collection()
        .expect("Failed to get collection")
        .list_indexes().await?;

    let mut found = false;

    while let Some(index) = cursor.try_next().await? {
        if let Some(opts) = index.options {
            if opts.name.as_deref() == Some("text_v2_idx") {
                if let Some(TextIndexVersion::V2) = opts.text_index_version {
                    found = true;
                }
            }
        }
    }

    assert!(found, "Expected index with name 'text_v2_idx'");

    Ok(())
}

// Run test: cargo nextest run hidden_index_is_applied_correctly
#[tokio::test]
async fn hidden_index_is_applied_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("hidden_index_test")]
    struct HiddenTest {
        #[index(hidden, name = "hidden_idx")]
        secret: String,
    }

    HiddenTest::clear().await?;

    let doc = HiddenTest::default().secret("classified".to_string());
    doc.save().await?;

    let mut cursor = HiddenTest::get_collection()
        .expect("failed to get collection")
        .list_indexes().await?;

    let mut found = false;

    while let Some(index) = cursor.try_next().await? {
        if let Some(name) = index.options.as_ref().and_then(|opts| opts.name.as_ref()) {
            if name == "hidden_idx" {
                let hidden = index.options.as_ref().and_then(|opts| opts.hidden);
                assert_eq!(hidden, Some(true));
                found = true;
                break;
            }
        }
    }

    assert!(found, "Expected index 'hidden_idx' not found");
    Ok(())
}
