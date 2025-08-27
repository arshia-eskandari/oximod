use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, DateTime},
    options::{IndexVersion, TextIndexVersion},
};
use oximod::Model;
use serde::{Deserialize, Serialize};
use std::{thread::sleep, time::Duration};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run creates_indexes_correctly
#[tokio::test]
async fn creates_indexes_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_test_creates_indexes_correctly")]
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
    #[collection("ttl_test_removes_expired_documents")]
    pub struct Session {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(expire_after_secs = 2)]
        created_at: Option<DateTime>,
    }

    Session::clear().await?;

    let expired_session = Session::default().created_at(DateTime::from_millis(
        DateTime::now().timestamp_millis() - 10_000,
    ));

    expired_session.save().await?;

    sleep(Duration::from_secs(65));

    let remaining = Session::find(doc! {}).await?;
    assert_eq!(
        remaining.len(),
        0,
        "Expected document to be expired and deleted"
    );

    Ok(())
}

// Run test: cargo nextest run index_version_is_applied_correctly
#[tokio::test]
async fn index_version_is_applied_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_version_is_applied_correctly")]
    pub struct VersionedIndex {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(version = 2, name = "v2_idx")]
        data: String,
    }

    VersionedIndex::clear().await?;

    let item = VersionedIndex::default().data("hello".to_string());
    item.save().await?;

    let mut cursor = VersionedIndex::get_collection()?.list_indexes().await?;
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

    let mut cursor = TestModel::get_collection()?.list_indexes().await?;
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
    #[collection("hidden_index_is_applied_correctly")]
    struct HiddenTest {
        #[index(hidden, name = "hidden_idx")]
        secret: String,
    }

    HiddenTest::clear().await?;

    let doc = HiddenTest::default().secret("classified".to_string());
    doc.save().await?;

    let mut cursor = HiddenTest::get_collection()?.list_indexes().await?;
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

// Run test: cargo nextest run creates_indexes_correctly_fails_on_duplicate
#[tokio::test]
async fn creates_indexes_correctly_fails_on_duplicate() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_test_duplicate_fails")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "name_idx")]
        name: String,
    }

    User::clear().await?;

    let user1 = User::default().name("IndexUser".to_string());
    let user2 = User::default().name("IndexUser".to_string());

    user1.save().await?;

    let dup_result = user2.save().await;
    assert!(
        dup_result.is_err(),
        "Expected duplicate unique index to fail"
    );

    Ok(())
}

// Run test: cargo nextest run index_init_respects_overridden_retry_and_timeout
#[tokio::test]
async fn index_init_respects_overridden_retry_and_timeout() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_init_overrides")]
    #[index_max_retries(7)]
    #[index_max_init_seconds(45)]
    pub struct UserOverride {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "overrides_name_idx")]
        name: String,
    }

    UserOverride::clear().await?;

    let doc = UserOverride::default().name("User1".to_string());
    let result = doc.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}
