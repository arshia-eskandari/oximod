//! Integration tests for explicit, write-independent index initialization.
//!
//! `init_indexes()` and `init_indexes_from(...)` establish a collection
//! model's declared `#[index(...)]` specifications without saving any
//! document. They reuse the save path's index machinery and share its
//! once-per-process establishment state.
//!
//! Every test uses its own model type and collection: index initialization
//! state is process-local and per model type, so distinct types keep the
//! tests order-independent.

use futures_util::TryStreamExt;
use mongodb::{
    Collection, IndexModel,
    bson::{Bson, DateTime, doc, oid::ObjectId},
};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

async fn find_index_in<T>(
    collection: &Collection<T>,
    name: &str,
) -> Result<Option<IndexModel>, Box<dyn std::error::Error>>
where
    T: Send + Sync,
{
    let mut cursor = collection.list_indexes().await?;

    while let Some(index) = cursor.try_next().await? {
        if index.options.as_ref().and_then(|opts| opts.name.as_deref()) == Some(name) {
            return Ok(Some(index));
        }
    }

    Ok(None)
}

fn assert_key_is(index: &IndexModel, field: &str, expected: Bson) {
    let actual = index.keys.get(field);
    assert_eq!(
        actual,
        Some(&expected),
        "expected index key `{}` to be {:?}, got {:?}",
        field,
        expected,
        actual
    );
}

// Run test: cargo nextest run init_indexes_establishes_declared_indexes_without_saving
#[tokio::test]
async fn init_indexes_establishes_declared_indexes_without_saving() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("init_indexes_save_free")]
    pub struct SaveFreeInit {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "sfi_code_uidx")]
        code: String,

        #[index(sparse, order = "-1", name = "sfi_rank_idx")]
        rank: Option<i32>,

        #[index(expire_after_secs = 3600, name = "sfi_ttl_idx")]
        created_at: Option<DateTime>,
    }

    let collection = SaveFreeInit::get_collection()?;
    let _ = collection.drop().await;

    // No save() has run for this model; establishment is explicit only.
    SaveFreeInit::init_indexes().await?;

    let unique_index = find_index_in(&collection, "sfi_code_uidx")
        .await?
        .expect("expected `sfi_code_uidx` to exist without any save");
    assert_key_is(&unique_index, "code", Bson::Int32(1));
    assert_eq!(
        unique_index.options.as_ref().and_then(|opts| opts.unique),
        Some(true),
        "expected `sfi_code_uidx` to be unique"
    );

    let sparse_index = find_index_in(&collection, "sfi_rank_idx")
        .await?
        .expect("expected `sfi_rank_idx` to exist without any save");
    assert_key_is(&sparse_index, "rank", Bson::Int32(-1));
    assert_eq!(
        sparse_index.options.as_ref().and_then(|opts| opts.sparse),
        Some(true),
        "expected `sfi_rank_idx` to be sparse"
    );

    // TTL fidelity is proven from the server-side index specification; no
    // TTL-monitor deletion wait is involved.
    let ttl_index = find_index_in(&collection, "sfi_ttl_idx")
        .await?
        .expect("expected `sfi_ttl_idx` to exist without any save");
    assert_key_is(&ttl_index, "created_at", Bson::Int32(1));
    assert_eq!(
        ttl_index
            .options
            .as_ref()
            .and_then(|opts| opts.expire_after)
            .map(|duration| duration.as_secs()),
        Some(3600),
        "expected `sfi_ttl_idx` TTL to be 3600 seconds"
    );

    let documents = collection.count_documents(doc! {}).await?;
    assert_eq!(
        documents, 0,
        "index initialization must not write any model document"
    );

    Ok(())
}

// Run test: cargo nextest run unique_constraint_is_active_before_first_oximod_save
#[tokio::test]
async fn unique_constraint_is_active_before_first_oximod_save() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("init_indexes_unique_before_save")]
    pub struct UniqueBeforeSave {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "ubs_code_uidx")]
        code: String,
    }

    let collection = UniqueBeforeSave::get_collection()?;
    let _ = collection.drop().await;

    UniqueBeforeSave::init_indexes().await?;

    // Seed server state through the raw document collection: raw inserts do
    // not trigger OxiMod's save-based index initialization.
    let raw = UniqueBeforeSave::get_document_collection()?;
    raw.insert_one(doc! { "code": "A" }).await?;
    let second = raw.insert_one(doc! { "code": "B" }).await?;
    let second_id = second
        .inserted_id
        .as_object_id()
        .expect("raw insert should produce an ObjectId");

    // A typed update that would violate the declared unique index must be
    // rejected by MongoDB before any OxiMod save has run for this model.
    let error = UniqueBeforeSave::update_by_id(second_id, doc! { "$set": { "code": "A" } })
        .await
        .expect_err("duplicate-producing typed update should be rejected by the unique index");

    let source = std::error::Error::source(&error)
        .expect("update rejection should preserve its driver source");
    let driver = source
        .downcast_ref::<mongodb::error::Error>()
        .expect("source should be a mongodb::error::Error");
    assert!(
        format!("{driver:?}").contains("11000"),
        "expected a duplicate-key (E11000) rejection, got {driver:?}"
    );

    let duplicates = collection.count_documents(doc! { "code": "A" }).await?;
    assert_eq!(duplicates, 1, "the duplicate value must not have persisted");

    Ok(())
}

// Run test: cargo nextest run repeated_initialization_is_harmless
#[tokio::test]
async fn repeated_initialization_is_harmless() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("init_indexes_repeated")]
    pub struct RepeatedInit {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "ri_code_uidx")]
        code: String,
    }

    let collection = RepeatedInit::get_collection()?;
    let _ = collection.drop().await;

    RepeatedInit::init_indexes().await?;
    RepeatedInit::init_indexes()
        .await
        .expect("repeated successful initialization should be a harmless no-op");

    let mut cursor = collection.list_indexes().await?;
    let mut matching = 0;
    while let Some(index) = cursor.try_next().await? {
        if index.options.as_ref().and_then(|opts| opts.name.as_deref()) == Some("ri_code_uidx") {
            matching += 1;
        }
    }
    assert_eq!(matching, 1, "expected exactly one `ri_code_uidx` index");

    Ok(())
}

// Run test: cargo nextest run init_and_save_share_once_per_process_state
#[tokio::test]
async fn init_and_save_share_once_per_process_state() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("init_indexes_shared_state")]
    pub struct SharedOnce {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "shared_once_idx")]
        tag: String,
    }

    let collection = SharedOnce::get_collection()?;
    let _ = collection.drop().await;

    SharedOnce::init_indexes().await?;
    assert!(
        find_index_in(&collection, "shared_once_idx")
            .await?
            .is_some(),
        "explicit initialization should establish `shared_once_idx`"
    );

    // Remove the established index out-of-band, then save. Establishment is
    // once per process and the explicit path shares that state with the save
    // path, so the save must NOT re-run index creation. This pins the
    // documented no-re-establishment lifecycle; drift repair is out of scope
    // and not expected here.
    collection.drop_index("shared_once_idx").await?;

    SharedOnce::default().tag("t1").save().await?;

    assert!(
        find_index_in(&collection, "shared_once_idx")
            .await?
            .is_none(),
        "save after a successful explicit initialization must not re-establish \
         the index; the once-per-process state is shared"
    );

    Ok(())
}

// Run test: cargo nextest run init_indexes_from_uses_the_supplied_client
//
// This test deliberately does not call `common::init`: under process-per-test
// execution the global client is never installed, so success here proves
// `init_indexes_from` operates entirely through the supplied client.
#[tokio::test]
async fn init_indexes_from_uses_the_supplied_client() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");

    let owner = OxiClient::new(mongodb_uri).await?;
    let client = owner
        .client()
        .expect("OxiClient::new initializes its client");

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("init_indexes_explicit_client")]
    pub struct ExplicitClientInit {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "eci_email_uidx")]
        email: String,
    }

    let collection = ExplicitClientInit::get_collection_from(client)?;
    let _ = collection.drop().await;

    ExplicitClientInit::init_indexes_from(client).await?;

    let index = find_index_in(&collection, "eci_email_uidx")
        .await?
        .expect("expected `eci_email_uidx` to be established through the supplied client");
    assert_key_is(&index, "email", Bson::Int32(1));
    assert_eq!(
        index.options.as_ref().and_then(|opts| opts.unique),
        Some(true),
        "expected `eci_email_uidx` to be unique"
    );

    let documents = collection.count_documents(doc! {}).await?;
    assert_eq!(
        documents, 0,
        "explicit-client initialization must not write any model document"
    );

    Ok(())
}

// Run test: cargo nextest run init_indexes_failure_uses_index_error_surface_and_can_retry
#[tokio::test]
async fn init_indexes_failure_uses_index_error_surface_and_can_retry() -> TestResult {
    init().await?;

    // Two models sharing one collection may conflict at the server: the same
    // index name over different keys is rejected with IndexKeySpecsConflict.
    // This is a legitimate runtime establishment failure that does not rely
    // on declarations rejected at compile time.
    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("init_indexes_error_surface")]
    pub struct FirstConflict {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "iies_clash_idx")]
        field_a: String,
    }

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("init_indexes_error_surface")]
    pub struct SecondConflict {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "iies_clash_idx")]
        field_b: String,
    }

    let collection = FirstConflict::get_collection()?;
    let _ = collection.drop().await;

    FirstConflict::init_indexes().await?;

    let error = SecondConflict::init_indexes()
        .await
        .expect_err("a conflicting server-side index name should fail initialization");
    assert!(
        matches!(error, oximod::OxiModError::Index { .. }),
        "expected OxiModError::Index, got {error:?}"
    );
    assert!(
        error.to_string().contains("`init_indexes_error_surface`"),
        "index error should name the collection: {error}"
    );
    assert!(
        std::error::Error::source(&error).is_some(),
        "index error should preserve its driver source: {error:?}"
    );

    // A failed attempt must not complete the once-per-process state: the next
    // call retries and fails the same way while the conflict persists.
    let retry_error = SecondConflict::init_indexes()
        .await
        .expect_err("initialization should retry, and fail again, while the conflict persists");
    assert!(
        matches!(retry_error, oximod::OxiModError::Index { .. }),
        "expected OxiModError::Index on retry, got {retry_error:?}"
    );

    // Once the server-side conflict is removed, the existing failure-reset
    // semantics allow a later attempt to succeed.
    collection.drop_index("iies_clash_idx").await?;

    SecondConflict::init_indexes()
        .await
        .expect("initialization should succeed after the conflicting index is removed");

    let index = find_index_in(&collection, "iies_clash_idx")
        .await?
        .expect("expected `iies_clash_idx` to exist after the successful retry");
    assert_key_is(&index, "field_b", Bson::Int32(1));

    Ok(())
}
