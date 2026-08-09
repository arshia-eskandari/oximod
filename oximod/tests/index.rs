//! Integration tests for MongoDB indexes generated from `#[index(...)]`.
//!
//! The tests cover scalar, unique, sparse, TTL, text, hashed, hidden,
//! case-insensitive, and geospatial indexes, along with version and retry options.

use futures_util::TryStreamExt;
use mongodb::{
    IndexModel,
    bson::{Bson, DateTime, doc, oid::ObjectId},
    options::{CollationStrength, IndexVersion, Sphere2DIndexVersion, TextIndexVersion},
};
use oximod::Model;
use serde::{Deserialize, Serialize};
use std::{thread::sleep, time::Duration};
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

async fn find_index_by_name<T>(name: &str) -> Result<Option<IndexModel>, Box<dyn std::error::Error>>
where
    T: Model,
{
    let mut cursor = T::get_collection()?.list_indexes().await?;

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

fn assert_option_name(index: &IndexModel, expected_name: &str) {
    let actual = index.options.as_ref().and_then(|opts| opts.name.as_deref());
    assert_eq!(actual, Some(expected_name), "unexpected index name");
}

fn assert_text_index_shape(index: &IndexModel) {
    assert_eq!(
        index.keys.get("_fts"),
        Some(&Bson::String("text".to_string())),
        "expected MongoDB text index marker `_fts: \"text\"`"
    );
    assert_eq!(
        index.keys.get("_ftsx"),
        Some(&Bson::Int32(1)),
        "expected MongoDB text index marker `_ftsx: 1`"
    );
}

// Run test: cargo nextest run creates_indexes_correctly
#[tokio::test]
async fn creates_indexes_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_test_creates_indexes_correctly")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "name_idx")]
        name: String,

        #[index(sparse, order = "-1", name = "age_desc_sparse_idx")]
        age: Option<i32>,

        #[index(expire_after_secs = 3600, name = "created_at_ttl_idx")]
        created_at: Option<DateTime>,

        active: bool,
    }

    reset_collection::<User>().await?;

    let user = User::default()
        .name("IndexUser")
        .age(25)
        .created_at(DateTime::now())
        .active(true);

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    let name_index = find_index_by_name::<User>("name_idx")
        .await?
        .expect("Expected index `name_idx` to exist");
    assert_option_name(&name_index, "name_idx");
    assert_key_is(&name_index, "name", Bson::Int32(1));
    assert_eq!(
        name_index.options.as_ref().and_then(|opts| opts.unique),
        Some(true),
        "Expected `name_idx` to be unique"
    );

    let age_index = find_index_by_name::<User>("age_desc_sparse_idx")
        .await?
        .expect("Expected index `age_desc_sparse_idx` to exist");
    assert_option_name(&age_index, "age_desc_sparse_idx");
    assert_key_is(&age_index, "age", Bson::Int32(-1));
    assert_eq!(
        age_index.options.as_ref().and_then(|opts| opts.sparse),
        Some(true),
        "Expected `age_desc_sparse_idx` to be sparse"
    );

    let ttl_index = find_index_by_name::<User>("created_at_ttl_idx")
        .await?
        .expect("Expected index `created_at_ttl_idx` to exist");
    assert_option_name(&ttl_index, "created_at_ttl_idx");
    assert_key_is(&ttl_index, "created_at", Bson::Int32(1));
    assert_eq!(
        ttl_index
            .options
            .as_ref()
            .and_then(|opts| opts.expire_after)
            .map(|d| d.as_secs()),
        Some(3600),
        "Expected TTL to be 3600 seconds"
    );

    Ok(())
}

// Run test: cargo nextest run ttl_index_removes_expired_documents
#[tokio::test]
async fn ttl_index_removes_expired_documents() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ttl_test_removes_expired_documents")]
    pub struct Session {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(expire_after_secs = 2, name = "session_ttl_idx")]
        created_at: Option<DateTime>,
    }

    reset_collection::<Session>().await?;

    let expired_session = Session::default().created_at(DateTime::from_millis(
        DateTime::now().timestamp_millis() - 10_000,
    ));

    expired_session.save().await?;

    let ttl_index = find_index_by_name::<Session>("session_ttl_idx")
        .await?
        .expect("Expected TTL index `session_ttl_idx` to exist");
    assert_option_name(&ttl_index, "session_ttl_idx");
    assert_key_is(&ttl_index, "created_at", Bson::Int32(1));
    assert_eq!(
        ttl_index
            .options
            .as_ref()
            .and_then(|opts| opts.expire_after)
            .map(|d| d.as_secs()),
        Some(2),
        "Expected TTL to be 2 seconds"
    );

    sleep(Duration::from_secs(65));

    let collection = Session::get_collection()?;
    let remaining = collection.count_documents(doc! {}).await?;
    assert_eq!(remaining, 0, "Expected document to be expired and deleted");

    Ok(())
}

// Run test: cargo nextest run index_version_is_applied_correctly
#[tokio::test]
async fn index_version_is_applied_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_version_is_applied_correctly")]
    pub struct VersionedIndex {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(version = 2, name = "v2_idx")]
        data: String,
    }

    reset_collection::<VersionedIndex>().await?;

    let item = VersionedIndex::default().data("hello");
    item.save().await?;

    let index = find_index_by_name::<VersionedIndex>("v2_idx")
        .await?
        .expect("Expected index with name `v2_idx`");
    assert_option_name(&index, "v2_idx");
    assert_key_is(&index, "data", Bson::Int32(1));

    let version = index.options.as_ref().and_then(|opts| opts.version.clone());
    assert!(
        matches!(version, Some(IndexVersion::V2)),
        "Expected `v2_idx` to use IndexVersion::V2, got {:?}",
        version
    );

    Ok(())
}

// Run test: cargo nextest run text_index_version_is_applied_correctly
#[tokio::test]
async fn text_index_version_is_applied_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("text_index_version_is_applied_correctly")]
    pub struct TestModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(text_index_version = 2, name = "text_v2_idx")]
        data: String,
    }

    reset_collection::<TestModel>().await?;

    let item = TestModel::default().data("hello");
    item.save().await?;

    let index = find_index_by_name::<TestModel>("text_v2_idx")
        .await?
        .expect("Expected index with name `text_v2_idx`");
    assert_option_name(&index, "text_v2_idx");
    assert_text_index_shape(&index);

    let text_options = index
        .options
        .as_ref()
        .expect("Expected `text_v2_idx` to have options");

    let text_index_version = text_options.text_index_version.clone();
    assert!(
        matches!(text_index_version, Some(TextIndexVersion::V2)),
        "Expected `text_v2_idx` to use TextIndexVersion::V2, got {:?}",
        text_index_version
    );

    let weights = text_options
        .weights
        .as_ref()
        .expect("Expected `text_v2_idx` to have weights");
    assert_eq!(
        weights.get("data"),
        Some(&Bson::Int32(1)),
        "Expected MongoDB to register `data` in text index weights"
    );

    Ok(())
}

// Run test: cargo nextest run hidden_index_is_applied_correctly
#[tokio::test]
async fn hidden_index_is_applied_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("hidden_index_is_applied_correctly")]
    struct HiddenTest {
        #[index(hidden, name = "hidden_idx")]
        secret: String,
    }

    reset_collection::<HiddenTest>().await?;

    let doc = HiddenTest::default().secret("classified");
    doc.save().await?;

    let index = find_index_by_name::<HiddenTest>("hidden_idx")
        .await?
        .expect("Expected index `hidden_idx` not found");
    assert_option_name(&index, "hidden_idx");
    assert_key_is(&index, "secret", Bson::Int32(1));
    assert_eq!(
        index.options.as_ref().and_then(|opts| opts.hidden),
        Some(true),
        "Expected `hidden_idx` to be hidden"
    );

    Ok(())
}

// Run test: cargo nextest run creates_indexes_correctly_fails_on_duplicate
#[tokio::test]
async fn creates_indexes_correctly_fails_on_duplicate() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_test_duplicate_fails")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "name_idx")]
        name: String,
    }

    reset_collection::<User>().await?;

    let user1 = User::default().name("IndexUser");
    user1.save().await?;

    let index = find_index_by_name::<User>("name_idx")
        .await?
        .expect("Expected index `name_idx` to exist");
    assert_option_name(&index, "name_idx");
    assert_key_is(&index, "name", Bson::Int32(1));
    assert_eq!(
        index.options.as_ref().and_then(|opts| opts.unique),
        Some(true),
        "Expected `name_idx` to be unique"
    );

    let user2 = User::default().name("IndexUser");
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
    init().await?;

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

    reset_collection::<UserOverride>().await?;

    let doc = UserOverride::default().name("User1");
    let result = doc.save().await?;
    assert_ne!(result, ObjectId::default());

    let index = find_index_by_name::<UserOverride>("overrides_name_idx")
        .await?
        .expect("Expected index `overrides_name_idx` to exist");
    assert_option_name(&index, "overrides_name_idx");
    assert_key_is(&index, "name", Bson::Int32(1));

    Ok(())
}

// Run test: cargo nextest run advanced_index_features_are_applied_correctly
#[tokio::test]
async fn advanced_index_features_are_applied_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("advanced_index_features")]
    struct Advanced {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(text, weight = 5, default_language = "english", name = "text_idx")]
        title: String,

        #[index(hashed, name = "hashed_idx")]
        user_id: String,

        #[index(case_insensitive, name = "ci_idx")]
        email: String,

        #[index(geo_2dsphere, geo_2dsphere_index_version = 3, name = "geo_idx")]
        location: Vec<f64>,
    }

    reset_collection::<Advanced>().await?;

    let doc = Advanced::default()
        .title("hello world")
        .user_id("user123")
        .email("TEST@EMAIL.COM")
        .location(vec![0.0, 0.0]);

    doc.save().await?;

    let text_index = find_index_by_name::<Advanced>("text_idx")
        .await?
        .expect("Expected index `text_idx` to exist");
    assert_option_name(&text_index, "text_idx");
    assert_text_index_shape(&text_index);

    let text_options = text_index
        .options
        .as_ref()
        .expect("Expected `text_idx` to have options");
    assert_eq!(
        text_options.default_language.as_deref(),
        Some("english"),
        "Expected `text_idx` default language to be `english`"
    );
    let weights = text_options
        .weights
        .as_ref()
        .expect("Expected `text_idx` to have weights");
    assert_eq!(
        weights.get("title"),
        Some(&Bson::Int32(5)),
        "Expected `title` weight to be 5"
    );

    let hashed_index = find_index_by_name::<Advanced>("hashed_idx")
        .await?
        .expect("Expected index `hashed_idx` to exist");
    assert_option_name(&hashed_index, "hashed_idx");
    assert_key_is(&hashed_index, "user_id", Bson::String("hashed".to_string()));

    let ci_index = find_index_by_name::<Advanced>("ci_idx")
        .await?
        .expect("Expected index `ci_idx` to exist");
    assert_option_name(&ci_index, "ci_idx");
    assert_key_is(&ci_index, "email", Bson::Int32(1));

    let collation = ci_index
        .options
        .as_ref()
        .and_then(|opts| opts.collation.as_ref())
        .expect("Expected `ci_idx` to have collation");
    assert_eq!(
        collation.locale, "en",
        "Expected `ci_idx` collation locale to be `en`"
    );
    assert!(
        matches!(
            collation.strength.as_ref(),
            Some(CollationStrength::Secondary)
        ),
        "Expected `ci_idx` collation strength to be Secondary"
    );

    let geo_index = find_index_by_name::<Advanced>("geo_idx")
        .await?
        .expect("Expected index `geo_idx` to exist");
    assert_option_name(&geo_index, "geo_idx");
    assert_key_is(&geo_index, "location", Bson::String("2dsphere".to_string()));

    let sphere_version = geo_index
        .options
        .as_ref()
        .and_then(|opts| opts.sphere_2d_index_version.clone());
    assert!(
        matches!(sphere_version, Some(Sphere2DIndexVersion::V3)),
        "Expected `geo_idx` to use Sphere2DIndexVersion::V3, got {:?}",
        sphere_version
    );

    Ok(())
}

// Run test: cargo nextest run index_creation_errors_name_the_collection
#[tokio::test]
async fn index_creation_errors_name_the_collection() -> TestResult {
    init().await?;

    // Two models sharing one collection may still conflict at the server:
    // the same index name over different keys is rejected with
    // IndexOptionsConflict. This is deliberately not a compile-time conflict,
    // so it exercises the runtime index-creation error surface.
    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_error_names_collection")]
    struct FirstModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "clash_idx")]
        first_field: String,
    }

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("index_error_names_collection")]
    struct SecondModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "clash_idx")]
        second_field: String,
    }

    reset_collection::<FirstModel>().await?;

    FirstModel::default().first_field("a").save().await?;

    let error = SecondModel::default()
        .second_field("b")
        .save()
        .await
        .expect_err("conflicting server-side index name should fail the save");

    assert!(
        matches!(error, oximod::OxiModError::Index { .. }),
        "expected OxiModError::Index, got {error:?}"
    );
    assert!(
        error.to_string().contains("`index_error_names_collection`"),
        "index error should name the collection: {error}"
    );
    assert!(
        std::error::Error::source(&error).is_some(),
        "index error should preserve its driver source: {error:?}"
    );

    Ok(())
}
