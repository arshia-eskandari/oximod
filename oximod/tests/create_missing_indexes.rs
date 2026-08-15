//! Integration tests for conservative create-only index reconciliation.
//!
//! `create_missing_indexes()` and `create_missing_indexes_from(...)` create
//! only declared `#[index(...)]` specifications classified Missing.
//! Mismatched declarations and unmanaged raw-driver indexes are never
//! modified, and the once-per-process establishment state used by
//! `init_indexes*()` is neither consulted nor changed.
//!
//! Every test uses its own model type and collection so the tests stay
//! order-independent.

use std::error::Error as StdError;
use std::time::Duration;

use mongodb::{
    Collection, IndexModel,
    bson::{Bson, DateTime, Document, doc, oid::ObjectId},
    options::IndexOptions,
};
use oximod::{DeclaredIndexStatus, Model, OxiModError};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

async fn reset_collection<T>() -> Result<(), Box<dyn std::error::Error>>
where
    T: Model,
{
    let collection = T::get_collection()?;
    let _ = collection.drop().await;
    Ok(())
}

async fn collection_exists<T>(name: &str) -> Result<bool, Box<dyn std::error::Error>>
where
    T: Model,
{
    let names = T::get_collection()?
        .client()
        .database("test")
        .list_collection_names()
        .await?;
    Ok(names.iter().any(|candidate| candidate == name))
}

async fn find_index_in(
    collection: &Collection<Document>,
    name: &str,
) -> Result<Option<IndexModel>, Box<dyn std::error::Error>> {
    let mut cursor = collection.list_indexes().await?;

    while cursor.advance().await? {
        let index = cursor.deserialize_current()?;
        if index.options.as_ref().and_then(|opts| opts.name.as_deref()) == Some(name) {
            return Ok(Some(index));
        }
    }

    Ok(None)
}

// Run test: cargo nextest run missing_declarations_are_created_without_writing_documents
#[tokio::test]
async fn missing_declarations_are_created_without_writing_documents() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_missing_basic")]
    pub struct MissingBasic {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "mb_code_uidx")]
        code: String,

        #[index(sparse, order = "-1", name = "mb_rank_idx")]
        rank: Option<i32>,
    }

    reset_collection::<MissingBasic>().await?;

    let result = MissingBasic::create_missing_indexes().await?;

    // Before: everything missing on the absent collection. MongoDB's
    // createIndexes implicitly creates the collection; that is documented
    // and acceptable when declarations exist.
    assert_eq!(result.before().missing_count(), 2);
    assert_eq!(result.attempted_creations().len(), 2);
    assert!(result.after().is_in_sync());
    assert!(result.is_in_sync());

    let collection = MissingBasic::get_collection()?;
    assert_eq!(
        collection.count_documents(doc! {}).await?,
        0,
        "reconciliation must not write any model document"
    );

    let raw = MissingBasic::get_document_collection()?;
    assert!(find_index_in(&raw, "mb_code_uidx").await?.is_some());
    assert!(find_index_in(&raw, "mb_rank_idx").await?.is_some());

    Ok(())
}

// Run test: cargo nextest run no_index_model_creates_neither_indexes_nor_collection
#[tokio::test]
async fn no_index_model_creates_neither_indexes_nor_collection() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_no_indexes")]
    pub struct NoIndexes {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        note: String,
    }

    reset_collection::<NoIndexes>().await?;

    let result = NoIndexes::create_missing_indexes().await?;

    assert!(result.before().declared().is_empty());
    assert!(result.attempted_creations().is_empty());
    assert!(result.is_in_sync());
    assert!(
        !collection_exists::<NoIndexes>("ir_create_no_indexes").await?,
        "a model without declarations must not create its collection"
    );

    Ok(())
}

// Run test: cargo nextest run once_state_lifecycle_is_independent_of_reconciliation
//
// The mandatory lifecycle proof: explicit reconciliation detects and
// recreates an externally dropped index while `init_indexes_from` remains
// the documented once-per-process no-op in the same process.
#[tokio::test]
async fn once_state_lifecycle_is_independent_of_reconciliation() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");

    let owner = oximod::OxiClient::new(mongodb_uri).await?;
    let client = owner
        .client()
        .expect("OxiClient::new initializes its client");

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_lifecycle")]
    pub struct Lifecycle {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "lc_code_uidx")]
        code: String,
    }

    let collection = Lifecycle::get_collection_from(client)?;
    let _ = collection.drop().await;

    // 1. Establishment succeeds and completes the once-per-process state.
    Lifecycle::init_indexes_from(client).await?;

    // 2. The raw driver drops the declared index out-of-band.
    collection.drop_index("lc_code_uidx").await?;

    // 3. Repeated initialization remains the documented no-op: the shared
    //    once state is completed, so nothing is re-read or re-created.
    Lifecycle::init_indexes_from(client).await?;
    let raw = Lifecycle::get_document_collection_from(client)?;
    assert!(
        find_index_in(&raw, "lc_code_uidx").await?.is_none(),
        "init_indexes_from after a successful establishment must not \
         re-establish the dropped index"
    );

    // 4. Explicit inspection detects the drift the once state ignores.
    let report = Lifecycle::check_indexes_from(client).await?;
    assert_eq!(report.missing_count(), 1);

    // 5. Explicit reconciliation recreates only the missing declaration.
    let result = Lifecycle::create_missing_indexes_from(client).await?;
    assert_eq!(result.attempted_creations().len(), 1);

    // 6. The declaration is in sync again.
    assert!(result.after().is_in_sync());
    let report = Lifecycle::check_indexes_from(client).await?;
    assert!(report.is_in_sync());
    assert!(find_index_in(&raw, "lc_code_uidx").await?.is_some());

    Ok(())
}

// Run test: cargo nextest run reconciliation_does_not_complete_the_establishment_once_state
#[tokio::test]
async fn reconciliation_does_not_complete_the_establishment_once_state() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_no_once_completion")]
    pub struct NoOnceCompletion {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "noc_tag_idx")]
        tag: String,
    }

    reset_collection::<NoOnceCompletion>().await?;

    // Reconciliation creates the index without going through the OnceAsync
    // establishment path.
    let result = NoOnceCompletion::create_missing_indexes().await?;
    assert_eq!(result.attempted_creations().len(), 1);

    // Drop it again out-of-band.
    NoOnceCompletion::get_collection()?
        .drop_index("noc_tag_idx")
        .await?;

    // The first save still runs lazy establishment: reconciliation must not
    // have marked the once state complete.
    NoOnceCompletion::default().tag("t1").save().await?;

    let raw = NoOnceCompletion::get_document_collection()?;
    assert!(
        find_index_in(&raw, "noc_tag_idx").await?.is_some(),
        "the first save must still establish declared indexes; \
         reconciliation must not complete the establishment once state"
    );

    Ok(())
}

// Run test: cargo nextest run mismatched_declarations_are_never_modified
#[tokio::test]
async fn mismatched_declarations_are_never_modified() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_mismatch_untouched")]
    pub struct MismatchUntouched {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "mu_unique_idx")]
        code: String,

        #[index(expire_after_secs = 3600, name = "mu_ttl_idx")]
        created_at: Option<DateTime>,

        #[index(hidden, name = "mu_hidden_idx")]
        secret: String,
    }

    reset_collection::<MismatchUntouched>().await?;

    // Occupy each declared name with a drifted specification: not unique, a
    // different TTL, not hidden.
    let raw = MismatchUntouched::get_document_collection()?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "code": 1 })
            .options(
                IndexOptions::builder()
                    .name("mu_unique_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "created_at": 1 })
            .options(
                IndexOptions::builder()
                    .name("mu_ttl_idx".to_string())
                    .expire_after(Duration::from_secs(60))
                    .build(),
            )
            .build(),
    )
    .await?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "secret": 1 })
            .options(
                IndexOptions::builder()
                    .name("mu_hidden_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;

    let result = MismatchUntouched::create_missing_indexes().await?;

    // Nothing was missing, so no createIndexes command was sent and every
    // mismatch remains for a human to resolve.
    assert_eq!(result.before().mismatched_count(), 3);
    assert!(result.attempted_creations().is_empty());
    assert_eq!(result.after().mismatched_count(), 3);
    assert!(!result.is_in_sync());

    // The server indexes are byte-for-byte the drifted originals: no unique
    // conversion, no collMod TTL change, no hide/unhide.
    let unique_index = find_index_in(&raw, "mu_unique_idx")
        .await?
        .expect("`mu_unique_idx` must survive reconciliation");
    assert_eq!(
        unique_index.options.as_ref().and_then(|opts| opts.unique),
        None,
        "the non-unique index must not be converted to unique"
    );

    let ttl_index = find_index_in(&raw, "mu_ttl_idx")
        .await?
        .expect("`mu_ttl_idx` must survive reconciliation");
    assert_eq!(
        ttl_index
            .options
            .as_ref()
            .and_then(|opts| opts.expire_after)
            .map(|duration| duration.as_secs()),
        Some(60),
        "the drifted TTL must not be modified"
    );

    let hidden_index = find_index_in(&raw, "mu_hidden_idx")
        .await?
        .expect("`mu_hidden_idx` must survive reconciliation");
    assert_eq!(
        hidden_index.options.as_ref().and_then(|opts| opts.hidden),
        None,
        "the visible index must not be hidden"
    );

    Ok(())
}

// Run test: cargo nextest run mixed_drift_creates_only_the_missing_declaration
#[tokio::test]
async fn mixed_drift_creates_only_the_missing_declaration() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_mixed_drift")]
    pub struct MixedDrift {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "md_missing_idx")]
        alpha: String,

        #[index(unique, name = "md_mismatched_idx")]
        beta: String,
    }

    reset_collection::<MixedDrift>().await?;

    // Occupy only the second declaration's name with a non-unique variant.
    let raw = MixedDrift::get_document_collection()?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "beta": 1 })
            .options(
                IndexOptions::builder()
                    .name("md_mismatched_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;

    let result = MixedDrift::create_missing_indexes().await?;

    // The safe work happens; the mismatch is left for a human.
    assert_eq!(result.before().missing_count(), 1);
    assert_eq!(result.before().mismatched_count(), 1);
    assert_eq!(result.attempted_creations().len(), 1);
    assert_eq!(
        result.attempted_creations()[0].keys,
        doc! { "alpha": 1 },
        "only the missing declaration may be submitted"
    );

    assert!(matches!(
        result.after().declared()[0],
        DeclaredIndexStatus::InSync { .. }
    ));
    assert!(matches!(
        result.after().declared()[1],
        DeclaredIndexStatus::Mismatched { .. }
    ));
    assert!(!result.is_in_sync());

    // The conflicting index is untouched.
    let mismatched = find_index_in(&raw, "md_mismatched_idx")
        .await?
        .expect("the mismatched index must survive reconciliation");
    assert_eq!(
        mismatched.options.as_ref().and_then(|opts| opts.unique),
        None
    );

    Ok(())
}

// Run test: cargo nextest run unmanaged_raw_indexes_survive_reconciliation
#[tokio::test]
async fn unmanaged_raw_indexes_survive_reconciliation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_unmanaged_survive")]
    pub struct UnmanagedSurvive {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "us_code_idx")]
        code: String,
    }

    reset_collection::<UnmanagedSurvive>().await?;

    // A raw compound index for application needs; the declared index is
    // missing.
    let raw = UnmanagedSurvive::get_document_collection()?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "region": 1, "city": -1 })
            .options(
                IndexOptions::builder()
                    .name("us_compound_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;

    let result = UnmanagedSurvive::create_missing_indexes().await?;

    assert_eq!(result.attempted_creations().len(), 1);
    assert!(result.is_in_sync());
    assert_eq!(result.after().unmanaged().len(), 1);

    // The unmanaged index survives, visible and unmodified.
    let compound = find_index_in(&raw, "us_compound_idx")
        .await?
        .expect("the unmanaged raw index must survive reconciliation");
    assert_eq!(compound.options.as_ref().and_then(|opts| opts.hidden), None);
    assert_eq!(compound.keys, doc! { "region": 1, "city": -1 });

    Ok(())
}

// Run test: cargo nextest run unique_creation_over_duplicates_fails_with_index_error_and_can_retry
#[tokio::test]
async fn unique_creation_over_duplicates_fails_with_index_error_and_can_retry() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_unique_duplicates")]
    pub struct UniqueDuplicates {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "ud_code_uidx")]
        code: String,
    }

    reset_collection::<UniqueDuplicates>().await?;

    // Existing data violates the declared uniqueness.
    let raw = UniqueDuplicates::get_document_collection()?;
    raw.insert_one(doc! { "code": "A" }).await?;
    raw.insert_one(doc! { "code": "A" }).await?;

    let error = UniqueDuplicates::create_missing_indexes()
        .await
        .expect_err("a unique build over duplicate data should fail");

    assert!(
        matches!(error, OxiModError::Index { .. }),
        "expected OxiModError::Index, got {error:?}"
    );
    let source = error
        .source()
        .and_then(|source| source.downcast_ref::<mongodb::error::Error>())
        .expect("the driver error should remain available through source()");
    assert!(
        format!("{source:?}").contains("11000"),
        "expected a duplicate-key (E11000) build failure, got {source:?}"
    );

    // No once-state guards reconciliation: after the data is fixed, the same
    // call simply works.
    raw.delete_one(doc! { "code": "A" }).await?;

    let result = UniqueDuplicates::create_missing_indexes().await?;
    assert_eq!(result.attempted_creations().len(), 1);
    assert!(result.is_in_sync());

    // And a further call finds nothing to do while still running the full
    // inspection.
    let result = UniqueDuplicates::create_missing_indexes().await?;
    assert!(result.attempted_creations().is_empty());
    assert!(result.is_in_sync());

    Ok(())
}

// Run test: cargo nextest run create_missing_indexes_from_uses_the_supplied_client
//
// This test deliberately does not call `common::init`: under process-per-test
// execution the global client is never installed, so success here proves
// `create_missing_indexes_from` operates entirely through the supplied
// client.
#[tokio::test]
async fn create_missing_indexes_from_uses_the_supplied_client() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");

    let owner = oximod::OxiClient::new(mongodb_uri).await?;
    let client = owner
        .client()
        .expect("OxiClient::new initializes its client");

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_explicit_client")]
    pub struct ExplicitClientCreate {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "ecc_code_uidx")]
        code: String,
    }

    let collection = ExplicitClientCreate::get_collection_from(client)?;
    let _ = collection.drop().await;

    let result = ExplicitClientCreate::create_missing_indexes_from(client).await?;

    assert_eq!(result.attempted_creations().len(), 1);
    assert!(result.is_in_sync());
    assert_eq!(
        collection.count_documents(doc! {}).await?,
        0,
        "explicit-client reconciliation must not write any model document"
    );

    Ok(())
}

// Run test: cargo nextest run unreachable_deployment_classifies_as_connection
//
// This test does not require the global client or a reachable deployment.
#[tokio::test]
async fn unreachable_deployment_classifies_as_connection() -> TestResult {
    const UNREACHABLE_URI: &str =
        "mongodb://127.0.0.1:1/?serverSelectionTimeoutMS=300&connectTimeoutMS=300";

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_unreachable")]
    pub struct Unreachable {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "un_code_uidx")]
        code: String,
    }

    let client = mongodb::Client::with_uri_str(UNREACHABLE_URI).await?;

    let error = Unreachable::check_indexes_from(&client)
        .await
        .expect_err("inspection should fail against an unreachable server");
    assert!(
        matches!(error, OxiModError::Connection { .. }),
        "expected OxiModError::Connection from check, got {error:?}"
    );
    assert!(error.source().is_some());

    let error = Unreachable::create_missing_indexes_from(&client)
        .await
        .expect_err("reconciliation should fail against an unreachable server");
    assert!(
        matches!(error, OxiModError::Connection { .. }),
        "expected OxiModError::Connection from create-missing, got {error:?}"
    );
    assert!(error.source().is_some());

    Ok(())
}

// Run test: cargo nextest run concurrent_create_missing_calls_converge
//
// Race sanity: reconciliation is point-in-time, not atomic. Two concurrent
// calls for the same missing exact declaration may both submit it; MongoDB
// treats the second exact-equivalent create as a no-op, so both calls
// succeed and exactly one index exists. Conflicting concurrent changes may
// instead surface as an index-domain error; OxiMod adds no retry loop.
#[tokio::test]
async fn concurrent_create_missing_calls_converge() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");

    let owner = oximod::OxiClient::new(mongodb_uri).await?;
    let client = owner
        .client()
        .expect("OxiClient::new initializes its client");

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_create_concurrent")]
    pub struct Concurrent {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "cc_code_uidx")]
        code: String,
    }

    let collection = Concurrent::get_collection_from(client)?;
    let _ = collection.drop().await;

    let first_client = client.clone();
    let second_client = client.clone();

    let (first, second) = tokio::join!(
        Concurrent::create_missing_indexes_from(&first_client),
        Concurrent::create_missing_indexes_from(&second_client),
    );

    let first = first?;
    let second = second?;
    assert!(first.is_in_sync());
    assert!(second.is_in_sync());

    // Exactly one declared index exists regardless of interleaving.
    let raw = Concurrent::get_document_collection_from(client)?;
    let mut cursor = raw.list_indexes().await?;
    let mut matching = 0;
    while cursor.advance().await? {
        let index = cursor.deserialize_current()?;
        if index.options.as_ref().and_then(|opts| opts.name.as_deref()) == Some("cc_code_uidx") {
            matching += 1;
            assert_eq!(index.keys.get("code"), Some(&Bson::Int32(1)));
        }
    }
    assert_eq!(matching, 1, "expected exactly one `cc_code_uidx` index");

    let report = Concurrent::check_indexes_from(client).await?;
    assert!(report.is_in_sync());

    Ok(())
}
