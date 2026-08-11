//! Integration tests for the failure-class error contract.
//!
//! MongoDB driver errors produced while executing OxiMod database operations
//! are classified by failure class rather than by the method that was
//! executing. These tests route the same underlying failure through multiple
//! public call sites and assert that the variant is identical, and that the
//! original `mongodb::error::Error` remains available through
//! `std::error::Error::source()`.
//!
//! Tests that need an unreachable deployment use an explicit client with a
//! short server-selection timeout, so failures surface quickly without
//! touching the global client. The two scenarios that require a specific
//! global-client state live in their own integration-test binaries so that
//! every binary is deterministic under ordinary `cargo test`, where all of a
//! binary's tests share one process, as well as under nextest's
//! process-per-test model:
//!
//! - `error_classification_global_unreachable.rs` — the global client is
//!   initialized against an unreachable deployment;
//! - `error_classification_global_missing.rs` — the global client is never
//!   initialized.
//!
//! Tests in this binary that use the global client run on one shared,
//! never-dropped runtime: the global client's background tasks live on the
//! runtime that first initializes it, so a per-test runtime would tear them
//! down when the first such test finished.

use std::error::Error as StdError;
use std::sync::OnceLock;

use mongodb::{
    Client, IndexModel,
    bson::{Document, doc, oid::ObjectId},
    error::{ErrorKind, WriteFailure},
    options::IndexOptions,
};
use oximod::{Model, OxiClient, OxiModError, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;
use tokio::runtime::Runtime;

/// Unreachable deployment with a short server-selection timeout.
const UNREACHABLE_URI: &str =
    "mongodb://127.0.0.1:1/?serverSelectionTimeoutMS=300&connectTimeoutMS=300";

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Returns the shared runtime for tests that use the global client.
fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("failed to create the shared tokio runtime"))
}

/// Initializes the global client against `MONGODB_URI`.
///
/// Repeated initialization within one test process is tolerated so the suite
/// stays deterministic under ordinary `cargo test`.
async fn init() -> Result<(), OxiModError> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
    match OxiClient::init_global(mongodb_uri).await {
        Ok(()) | Err(OxiModError::GlobalClientInit { .. }) => Ok(()),
        Err(error) => Err(error),
    }
}

/// Returns the preserved `mongodb::error::Error` behind an `OxiModError`.
fn driver_source(error: &OxiModError) -> Option<&mongodb::error::Error> {
    error
        .source()
        .and_then(|source| source.downcast_ref::<mongodb::error::Error>())
}

/// Returns the server error code preserved behind an `OxiModError`.
///
/// Duplicate-key failures may surface through the driver as
/// `ErrorKind::Write` for plain writes or as `ErrorKind::Command` for
/// findAndModify-based operations; the code is recoverable from the
/// preserved driver error in both forms.
fn server_error_code(error: &OxiModError) -> Option<i32> {
    match &*driver_source(error)?.kind {
        ErrorKind::Write(WriteFailure::WriteError(write_error)) => Some(write_error.code),
        ErrorKind::Command(command_error) => Some(command_error.code),
        _ => None,
    }
}

/// Asserts that a driver-backed error is `Connection` and keeps its source.
fn assert_connection(operation: &str, error: &OxiModError) {
    assert!(
        matches!(error, OxiModError::Connection { .. }),
        "expected {operation} against an unreachable server to classify as \
         Connection, got: {error:?}"
    );
    assert!(
        driver_source(error).is_some(),
        "expected {operation} to preserve the mongodb driver error through \
         source(), got: {error:?}"
    );
}

/// Asserts that an error is `Serialization` caused by BSON deserialization.
fn assert_deserialization(operation: &str, error: &OxiModError) {
    assert!(
        matches!(error, OxiModError::Serialization { .. }),
        "expected {operation} over an undeserializable document to classify \
         as Serialization, got: {error:?}"
    );
    let source = driver_source(error).unwrap_or_else(|| {
        panic!(
            "expected {operation} to preserve the mongodb driver error \
             through source(), got: {error:?}"
        )
    });
    assert!(
        matches!(&*source.kind, ErrorKind::BsonDeserialization(_)),
        "expected {operation} to preserve a BsonDeserialization driver kind, \
         got: {source:?}"
    );
}

// Run test: cargo nextest run duplicate_key_via_save_classifies_as_database
#[test]
fn duplicate_key_via_save_classifies_as_database() -> TestResult {
    runtime().block_on(async {
        init().await?;

        #[derive(Model, Serialize, Deserialize, Debug)]
        #[db("test")]
        #[collection("error_class_dup_save")]
        pub struct DupSave {
            #[serde(skip_serializing_if = "Option::is_none")]
            _id: Option<ObjectId>,
            #[index(unique, name = "ec_dup_save_uidx")]
            email: String,
        }

        DupSave::clear().await?;

        DupSave::default().email("dup@example.com").save().await?;

        let error = DupSave::default()
            .email("dup@example.com")
            .save()
            .await
            .expect_err("saving a duplicate unique value should fail");

        assert!(
            matches!(error, OxiModError::Database { .. }),
            "expected a duplicate key through save() to classify as Database, \
             got: {error:?}"
        );
        assert_eq!(
            server_error_code(&error),
            Some(11000),
            "expected duplicate-key code 11000 to remain recoverable through \
             source(), got: {error:?}"
        );

        Ok(())
    })
}

// Run test: cargo nextest run duplicate_key_via_update_by_id_classifies_as_database
#[test]
fn duplicate_key_via_update_by_id_classifies_as_database() -> TestResult {
    runtime().block_on(async {
        init().await?;

        #[derive(Model, Serialize, Deserialize, Debug)]
        #[db("test")]
        #[collection("error_class_dup_update_by_id")]
        pub struct DupUpdate {
            #[serde(skip_serializing_if = "Option::is_none")]
            _id: Option<ObjectId>,
            #[index(unique, name = "ec_dup_update_uidx")]
            email: String,
        }

        DupUpdate::clear().await?;

        DupUpdate::default()
            .email("first@example.com")
            .save()
            .await?;
        let second_id = DupUpdate::default()
            .email("second@example.com")
            .save()
            .await?;

        let error =
            DupUpdate::update_by_id(second_id, doc! { "$set": { "email": "first@example.com" } })
                .await
                .expect_err("updating into a duplicate unique value should fail");

        assert!(
            matches!(error, OxiModError::Database { .. }),
            "expected a duplicate key through update_by_id() to classify as \
             Database, got: {error:?}"
        );
        assert_eq!(
            server_error_code(&error),
            Some(11000),
            "expected duplicate-key code 11000 to remain recoverable through \
             source(), got: {error:?}"
        );

        Ok(())
    })
}

// Run test: cargo nextest run duplicate_key_via_typed_update_one_classifies_as_database
#[test]
fn duplicate_key_via_typed_update_one_classifies_as_database() -> TestResult {
    runtime().block_on(async {
        init().await?;

        #[derive(Model, Serialize, Deserialize, Debug)]
        #[db("test")]
        #[collection("error_class_dup_typed_update")]
        pub struct DupTypedUpdate {
            #[serde(skip_serializing_if = "Option::is_none")]
            _id: Option<ObjectId>,
            #[index(unique, name = "ec_dup_typed_uidx")]
            email: String,
        }

        DupTypedUpdate::clear().await?;

        DupTypedUpdate::default()
            .email("first@example.com")
            .save()
            .await?;
        DupTypedUpdate::default()
            .email("second@example.com")
            .save()
            .await?;

        let error = DupTypedUpdate::query()
            .filter(|user| user.email.eq("second@example.com"))
            .update_one(|user| user.email.set("first@example.com"))
            .await
            .expect_err("updating into a duplicate unique value should fail");

        assert!(
            matches!(error, OxiModError::Database { .. }),
            "expected a duplicate key through query().update_one() to classify \
             as Database, got: {error:?}"
        );
        assert_eq!(
            server_error_code(&error),
            Some(11000),
            "expected duplicate-key code 11000 to remain recoverable through \
             source(), got: {error:?}"
        );

        Ok(())
    })
}

// Run test: cargo nextest run unreachable_server_explicit_client_operations_classify_as_connection
#[tokio::test]
async fn unreachable_server_explicit_client_operations_classify_as_connection() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("error_class_unreachable_explicit")]
    pub struct Unreachable {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
    }

    let client = Client::with_uri_str(UNREACHABLE_URI).await?;
    let id = ObjectId::new();

    let error = Unreachable::default()
        .name("User1")
        .save_from(&client)
        .await
        .expect_err("save_from should fail against an unreachable server");
    assert_connection("save_from", &error);

    let error = Unreachable::find_by_id_from(id, &client)
        .await
        .expect_err("find_by_id_from should fail against an unreachable server");
    assert_connection("find_by_id_from", &error);

    let error = Unreachable::update_by_id_from(id, doc! { "$set": { "name": "x" } }, &client)
        .await
        .expect_err("update_by_id_from should fail against an unreachable server");
    assert_connection("update_by_id_from", &error);

    let error = Unreachable::delete_by_id_from(id, &client)
        .await
        .expect_err("delete_by_id_from should fail against an unreachable server");
    assert_connection("delete_by_id_from", &error);

    let error = Unreachable::exists_from(doc! { "name": "User1" }, &client)
        .await
        .expect_err("exists_from should fail against an unreachable server");
    assert_connection("exists_from", &error);

    let error = Unreachable::count_from(doc! { "name": "User1" }, &client)
        .await
        .expect_err("count_from should fail against an unreachable server");
    assert_connection("count_from", &error);

    let error = Unreachable::clear_from(&client)
        .await
        .expect_err("clear_from should fail against an unreachable server");
    assert_connection("clear_from", &error);

    Ok(())
}

// Run test: cargo nextest run undeserializable_document_classifies_as_serialization_across_read_paths
#[test]
fn undeserializable_document_classifies_as_serialization_across_read_paths() -> TestResult {
    runtime().block_on(async {
        init().await?;

        #[derive(Model, Serialize, Deserialize, Debug)]
        #[db("test")]
        #[collection("error_class_poison_read")]
        pub struct PoisonRead {
            #[serde(skip_serializing_if = "Option::is_none")]
            _id: Option<ObjectId>,
            name: String,
            age: i32,
        }

        PoisonRead::clear().await?;

        // Raw-insert a document that matches the collection but cannot
        // deserialize as the model: `age` holds a string instead of an i32.
        let poison_id = ObjectId::new();
        PoisonRead::get_document_collection()?
            .insert_one(doc! {
                "_id": poison_id,
                "name": "Poison",
                "age": "not-a-number",
            })
            .await?;

        let error = PoisonRead::find_by_id(poison_id)
            .await
            .expect_err("find_by_id over an undeserializable document should fail");
        assert_deserialization("find_by_id", &error);

        let error = PoisonRead::query()
            .filter(|user| user.name.eq("Poison"))
            .first()
            .await
            .expect_err("query().first() over an undeserializable document should fail");
        assert_deserialization("query().first()", &error);

        let error = PoisonRead::query()
            .filter(|user| user.name.eq("Poison"))
            .all()
            .await
            .expect_err("query().all() over an undeserializable document should fail");
        assert_deserialization("query().all()", &error);

        Ok(())
    })
}

// Run test: cargo nextest run client_side_serialization_failure_via_save_classifies_as_serialization
#[test]
fn client_side_serialization_failure_via_save_classifies_as_serialization() -> TestResult {
    runtime().block_on(async {
        init().await?;

        #[derive(Model, Serialize, Deserialize, Debug)]
        #[db("test")]
        #[collection("error_class_ser_save")]
        pub struct BigNumber {
            #[serde(skip_serializing_if = "Option::is_none")]
            _id: Option<ObjectId>,
            big: u64,
        }

        // u64::MAX cannot be represented in BSON, so encoding fails client-side.
        let error = BigNumber::default()
            .big(u64::MAX)
            .save()
            .await
            .expect_err("saving an unencodable value should fail");

        assert!(
            matches!(error, OxiModError::Serialization { .. }),
            "expected a client-side BSON encoding failure through save() to \
             classify as Serialization, got: {error:?}"
        );
        let source = driver_source(&error)
            .unwrap_or_else(|| panic!("expected a preserved driver error, got: {error:?}"));
        assert!(
            matches!(&*source.kind, ErrorKind::BsonSerialization(_)),
            "expected a BsonSerialization driver kind, got: {source:?}"
        );

        Ok(())
    })
}

// Run test: cargo nextest run index_spec_conflict_classifies_as_index_naming_collection
#[test]
fn index_spec_conflict_classifies_as_index_naming_collection() -> TestResult {
    runtime().block_on(async {
        init().await?;

        #[derive(Model, Serialize, Deserialize, Debug)]
        #[db("test")]
        #[collection("error_class_index_conflict")]
        pub struct IndexConflict {
            #[serde(skip_serializing_if = "Option::is_none")]
            _id: Option<ObjectId>,
            #[index(name = "ec_conflict_idx")]
            status: String,
        }

        let client = OxiClient::global()?;

        // Establish an index with the declared name but a different key
        // specification out of band, so the declared spec conflicts
        // server-side.
        let raw = client
            .database("test")
            .collection::<Document>("error_class_index_conflict");
        raw.drop().await?;
        raw.create_index(
            IndexModel::builder()
                .keys(doc! { "other_field": 1 })
                .options(
                    IndexOptions::builder()
                        .name("ec_conflict_idx".to_string())
                        .build(),
                )
                .build(),
        )
        .await?;

        let error = IndexConflict::init_indexes_from(&client)
            .await
            .expect_err("initializing a conflicting index spec should fail");

        assert!(
            matches!(error, OxiModError::Index { .. }),
            "expected a server-side index-spec conflict to classify as Index, \
             got: {error:?}"
        );
        assert!(
            error.to_string().contains("error_class_index_conflict"),
            "expected the index error to name the collection, got: {error}"
        );
        let source = driver_source(&error)
            .unwrap_or_else(|| panic!("expected a preserved driver error, got: {error:?}"));
        assert!(
            matches!(&*source.kind, ErrorKind::Command(_)),
            "expected the server rejection to remain inspectable, got: {source:?}"
        );

        Ok(())
    })
}

// Run test: cargo nextest run index_establishment_against_unreachable_server_classifies_as_connection
#[tokio::test]
async fn index_establishment_against_unreachable_server_classifies_as_connection() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("error_class_index_unreachable")]
    pub struct IndexUnreachable {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        #[index(unique, name = "ec_unreachable_uidx")]
        code: String,
    }

    let client = Client::with_uri_str(UNREACHABLE_URI).await?;

    let error = IndexUnreachable::init_indexes_from(&client)
        .await
        .expect_err("index establishment should fail against an unreachable server");

    assert_connection("init_indexes_from", &error);

    Ok(())
}

// Run test: cargo nextest run validation_failure_via_save_remains_validation_without_driver_call
#[tokio::test]
async fn validation_failure_via_save_remains_validation_without_driver_call() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("error_class_validation")]
    pub struct Validated {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        #[validate(min_length = 5)]
        name: String,
    }

    // The unreachable client proves validation fails before any driver call:
    // a driver round trip would classify as Connection instead.
    let client = Client::with_uri_str(UNREACHABLE_URI).await?;

    let error = Validated::default()
        .name("ab")
        .save_from(&client)
        .await
        .expect_err("saving an invalid model should fail");

    assert!(
        matches!(error, OxiModError::Validation(_)),
        "expected a validation failure to remain Validation, got: {error:?}"
    );
    assert!(
        error.source().is_none(),
        "expected a validation failure to carry no driver source, got: {error:?}"
    );

    Ok(())
}

// Run test: cargo nextest run invalid_page_number_remains_query_error
#[test]
fn invalid_page_number_remains_query_error() -> TestResult {
    runtime().block_on(async {
        init().await?;

        #[derive(Model, Serialize, Deserialize, Debug)]
        #[db("test")]
        #[collection("error_class_query_config")]
        pub struct Paged {
            #[serde(skip_serializing_if = "Option::is_none")]
            _id: Option<ObjectId>,
            name: String,
        }

        let error = Paged::query()
            .page(0, 10)
            .all()
            .await
            .expect_err("page number zero should fail");

        assert!(
            matches!(
                error.query_error(),
                Some(oximod::QueryError::InvalidPageNumber { page: 0 })
            ),
            "expected an invalid page number to remain a Query error, got: {error:?}"
        );

        Ok(())
    })
}
