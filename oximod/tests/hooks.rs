mod common;

use common::init;
use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Hooks, Model};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use testresult::TestResult;

static PRE_SAVE_CALLS: AtomicUsize = AtomicUsize::new(0);
static POST_SAVE_CALLS: AtomicUsize = AtomicUsize::new(0);
static PRE_SAVE_MUT_CALLS: AtomicUsize = AtomicUsize::new(0);
static POST_SAVE_MUT_CALLS: AtomicUsize = AtomicUsize::new(0);
static PRE_FIND_CALLS: AtomicUsize = AtomicUsize::new(0);
static POST_FIND_CALLS: AtomicUsize = AtomicUsize::new(0);
static PRE_UPDATE_CALLS: AtomicUsize = AtomicUsize::new(0);
static POST_UPDATE_CALLS: AtomicUsize = AtomicUsize::new(0);
static PRE_DELETE_CALLS: AtomicUsize = AtomicUsize::new(0);
static POST_DELETE_CALLS: AtomicUsize = AtomicUsize::new(0);

fn reset_counters() {
    PRE_SAVE_CALLS.store(0, Ordering::SeqCst);
    POST_SAVE_CALLS.store(0, Ordering::SeqCst);
    PRE_SAVE_MUT_CALLS.store(0, Ordering::SeqCst);
    POST_SAVE_MUT_CALLS.store(0, Ordering::SeqCst);
    PRE_FIND_CALLS.store(0, Ordering::SeqCst);
    POST_FIND_CALLS.store(0, Ordering::SeqCst);
    PRE_UPDATE_CALLS.store(0, Ordering::SeqCst);
    POST_UPDATE_CALLS.store(0, Ordering::SeqCst);
    PRE_DELETE_CALLS.store(0, Ordering::SeqCst);
    POST_DELETE_CALLS.store(0, Ordering::SeqCst);
}

// Run test: cargo nextest run test_save_runs_pre_and_post_save_hooks
#[tokio::test]
async fn test_save_runs_pre_and_post_save_hooks() -> TestResult {
    init().await?;
    reset_counters();

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("hooks_test_save_runs_pre_and_post_save_hooks")]
    #[hooks]
    struct Log {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        message: String,
        normalized: bool,
    }

    #[async_trait::async_trait]
    impl Hooks for Log {
        async fn pre_save(&self) -> Result<(), oximod::OxiModError> {
            PRE_SAVE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn post_save(&self) -> Result<(), oximod::OxiModError> {
            POST_SAVE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    Log::clear().await?;

    let log = Log::default().message("system started").normalized(false);

    let id = log.save().await?;
    assert_ne!(id, ObjectId::default());
    assert_eq!(PRE_SAVE_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(POST_SAVE_CALLS.load(Ordering::SeqCst), 1);

    Ok(())
}

// Run test: cargo nextest run test_save_mut_runs_mut_hooks_and_mutates_state
#[tokio::test]
async fn test_save_mut_runs_mut_hooks_and_mutates_state() -> TestResult {
    init().await?;
    reset_counters();

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("hooks_test_save_mut_runs_mut_hooks_and_mutates_state")]
    #[hooks]
    struct Log {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        message: String,
        normalized: bool,
    }

    #[async_trait::async_trait]
    impl Hooks for Log {
        async fn pre_save_mut(&mut self) -> Result<(), oximod::OxiModError> {
            PRE_SAVE_MUT_CALLS.fetch_add(1, Ordering::SeqCst);
            self.message = self.message.trim().to_string();
            self.normalized = true;
            Ok(())
        }

        async fn post_save_mut(&self) -> Result<(), oximod::OxiModError> {
            POST_SAVE_MUT_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    Log::clear().await?;

    let mut log = Log::default()
        .message("   background worker started   ")
        .normalized(false);

    let id = log.save_mut().await?;
    assert_ne!(id, ObjectId::default());

    assert_eq!(PRE_SAVE_MUT_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(POST_SAVE_MUT_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(log.message, "background worker started");
    assert!(log.normalized);

    let found = Log::find_by_id(id).await?.expect("log should exist");
    assert_eq!(found.message, "background worker started");
    assert!(found.normalized);

    Ok(())
}

// Run test: cargo nextest run test_find_by_id_runs_pre_and_post_find_hooks
#[tokio::test]
async fn test_find_by_id_runs_pre_and_post_find_hooks() -> TestResult {
    init().await?;
    reset_counters();

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("hooks_test_find_by_id_runs_pre_and_post_find_hooks")]
    #[hooks]
    struct Log {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        message: String,
    }

    #[async_trait::async_trait]
    impl Hooks for Log {
        async fn pre_find(id: ObjectId) -> Result<(), oximod::OxiModError> {
            assert_ne!(id, ObjectId::default());
            PRE_FIND_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn post_find(result: &Option<Self>) -> Result<(), oximod::OxiModError> {
            assert!(result.is_some());
            POST_FIND_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    Log::clear().await?;

    let id = Log::default().message("hello").save().await?;
    let found = Log::find_by_id(id).await?;

    assert!(found.is_some());
    assert_eq!(PRE_FIND_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(POST_FIND_CALLS.load(Ordering::SeqCst), 1);

    Ok(())
}

// Run test: cargo nextest run test_update_by_id_runs_pre_and_post_update_hooks
#[tokio::test]
async fn test_update_by_id_runs_pre_and_post_update_hooks() -> TestResult {
    init().await?;
    reset_counters();

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("hooks_test_update_by_id_runs_pre_and_post_update_hooks")]
    #[hooks]
    struct Log {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        message: String,
    }

    #[async_trait::async_trait]
    impl Hooks for Log {
        async fn pre_update(
            id: ObjectId,
            update: &mongodb::bson::Document,
        ) -> Result<(), oximod::OxiModError> {
            assert_ne!(id, ObjectId::default());
            assert!(update.contains_key("$set"));
            PRE_UPDATE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn post_update(
            id: ObjectId,
            update: &mongodb::bson::Document,
        ) -> Result<(), oximod::OxiModError> {
            assert_ne!(id, ObjectId::default());
            assert!(update.contains_key("$set"));
            POST_UPDATE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    Log::clear().await?;

    let id = Log::default().message("before").save().await?;

    let result = Log::update_by_id(
        id,
        doc! {
            "$set": {
                "message": "after"
            }
        },
    )
    .await?;

    assert_eq!(result.matched_count, 1);
    assert_eq!(PRE_UPDATE_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(POST_UPDATE_CALLS.load(Ordering::SeqCst), 1);

    let found = Log::find_by_id(id).await?.expect("log should exist");
    assert_eq!(found.message, "after");

    Ok(())
}

// Run test: cargo nextest run test_delete_by_id_runs_pre_and_post_delete_hooks
#[tokio::test]
async fn test_delete_by_id_runs_pre_and_post_delete_hooks() -> TestResult {
    init().await?;
    reset_counters();

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("hooks_test_delete_by_id_runs_pre_and_post_delete_hooks")]
    #[hooks]
    struct Log {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        message: String,
    }

    #[async_trait::async_trait]
    impl Hooks for Log {
        async fn pre_delete(id: ObjectId) -> Result<(), oximod::OxiModError> {
            assert_ne!(id, ObjectId::default());
            PRE_DELETE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn post_delete(id: ObjectId) -> Result<(), oximod::OxiModError> {
            assert_ne!(id, ObjectId::default());
            POST_DELETE_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    Log::clear().await?;

    let id = Log::default().message("to be deleted").save().await?;

    let result = Log::delete_by_id(id).await?;
    assert_eq!(result.deleted_count, 1);
    assert_eq!(PRE_DELETE_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(POST_DELETE_CALLS.load(Ordering::SeqCst), 1);

    let found = Log::find_by_id(id).await?;
    assert!(found.is_none());

    Ok(())
}

// Run test: cargo nextest run test_pre_save_hook_error_aborts_save
#[tokio::test]
async fn test_pre_save_hook_error_aborts_save() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("hooks_test_pre_save_hook_error_aborts_save")]
    #[hooks]
    struct Log {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        message: String,
    }

    #[async_trait::async_trait]
    impl Hooks for Log {
        async fn pre_save(&self) -> Result<(), oximod::OxiModError> {
            Err(oximod::OxiModError::validation(
                "pre_save rejected the operation",
            ))
        }
    }

    Log::clear().await?;

    let log = Log::default().message("should fail");
    let err = log.save().await;

    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("pre_save rejected the operation"));

    let count = Log::count(doc! {}).await?;
    assert_eq!(count, 0);

    Ok(())
}

// Run test: cargo nextest run test_pre_save_custom_error_aborts_save
#[tokio::test]
async fn test_pre_save_custom_error_aborts_save() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("hooks_test_pre_save_custom_error_aborts_save")]
    #[hooks]
    struct Log {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        message: String,
    }

    #[async_trait::async_trait]
    impl Hooks for Log {
        async fn pre_save(&self) -> Result<(), oximod::OxiModError> {
            Err(oximod::OxiModError::custom(
                "pre_save rejected the operation with a custom error",
            ))
        }
    }

    Log::clear().await?;

    let log = Log::default().message("should fail");
    let err = log.save().await;

    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("pre_save rejected the operation with a custom error"));

    let count = Log::count(doc! {}).await?;
    assert_eq!(count, 0);

    Ok(())
}

// Run test: cargo nextest run test_pre_update_custom_error_aborts_update
#[tokio::test]
async fn test_pre_update_custom_error_aborts_update() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("hooks_test_pre_update_custom_error_aborts_update")]
    #[hooks]
    struct Log {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        message: String,
    }

    #[async_trait::async_trait]
    impl Hooks for Log {
        async fn pre_update(
            id: ObjectId,
            update: &mongodb::bson::Document,
        ) -> Result<(), oximod::OxiModError> {
            assert_ne!(id, ObjectId::default());
            assert!(update.contains_key("$set"));

            Err(oximod::OxiModError::custom(
                "pre_update rejected the operation with a custom error",
            ))
        }
    }

    Log::clear().await?;

    let id = Log::default().message("before").save().await?;

    let result = Log::update_by_id(
        id,
        doc! {
            "$set": {
                "message": "after"
            }
        },
    )
    .await;

    assert!(result.is_err());
    assert!(
        format!("{:?}", result).contains("pre_update rejected the operation with a custom error")
    );

    let found = Log::find_by_id(id).await?.expect("log should exist");
    assert_eq!(found.message, "before");

    Ok(())
}
