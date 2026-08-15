//! Integration tests for read-only index drift inspection.
//!
//! `check_indexes()` and `check_indexes_from(...)` compare a collection
//! model's declared `#[index(...)]` specifications against the index
//! metadata MongoDB reports, without creating the collection or any index.
//!
//! Every test uses its own model type and collection so the tests stay
//! order-independent.

use mongodb::{
    IndexModel,
    bson::{Bson, DateTime, doc, oid::ObjectId},
    options::{Collation, CollationStrength, IndexOptions},
};
use oximod::{DeclaredIndexStatus, Model};
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

fn status_name(status: &DeclaredIndexStatus) -> &'static str {
    match status {
        DeclaredIndexStatus::InSync { .. } => "InSync",
        DeclaredIndexStatus::Missing { .. } => "Missing",
        DeclaredIndexStatus::Mismatched { .. } => "Mismatched",
        _ => "unknown",
    }
}

fn assert_single_in_sync(report: &oximod::IndexDriftReport) {
    assert_eq!(report.declared().len(), 1, "expected one declared status");
    assert!(
        matches!(report.declared()[0], DeclaredIndexStatus::InSync { .. }),
        "expected InSync, got {:?}",
        report.declared()[0]
    );
    assert!(report.is_in_sync());
}

fn mismatch_differences(status: &DeclaredIndexStatus) -> Vec<String> {
    match status {
        DeclaredIndexStatus::Mismatched { candidates, .. } => candidates
            .iter()
            .flat_map(|candidate| candidate.differences().iter().cloned())
            .collect(),
        other => panic!("expected Mismatched, got {other:?}"),
    }
}

// Run test: cargo nextest run absent_collection_check_is_read_only_and_reports_missing
#[tokio::test]
async fn absent_collection_check_is_read_only_and_reports_missing() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_absent_collection")]
    pub struct AbsentCollection {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "acc_code_uidx")]
        code: String,

        #[index(order = "-1")]
        rank: i32,
    }

    reset_collection::<AbsentCollection>().await?;

    let report = AbsentCollection::check_indexes().await?;

    assert_eq!(report.declared().len(), 2);
    assert!(
        report
            .declared()
            .iter()
            .all(|status| matches!(status, DeclaredIndexStatus::Missing { .. })),
        "every declaration should be Missing for an absent collection"
    );
    assert_eq!(report.missing_count(), 2);
    assert!(report.unmanaged().is_empty());
    assert!(report.has_drift());

    // The inspection must not have created the collection.
    assert!(
        !collection_exists::<AbsentCollection>("ir_check_absent_collection").await?,
        "check_indexes must not create the collection"
    );

    Ok(())
}

// Run test: cargo nextest run absent_collection_without_declarations_is_in_sync
#[tokio::test]
async fn absent_collection_without_declarations_is_in_sync() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_no_indexes_absent")]
    pub struct NoIndexAbsent {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        note: String,
    }

    reset_collection::<NoIndexAbsent>().await?;

    let report = NoIndexAbsent::check_indexes().await?;

    assert!(report.declared().is_empty());
    assert!(report.unmanaged().is_empty());
    assert!(report.is_in_sync());
    assert!(!report.has_drift());
    assert!(
        !collection_exists::<NoIndexAbsent>("ir_check_no_indexes_absent").await?,
        "check_indexes must not create the collection"
    );

    Ok(())
}

// Run test: cargo nextest run check_indexes_from_uses_the_supplied_client
//
// This test deliberately does not call `common::init`: under process-per-test
// execution the global client is never installed, so success here proves
// `check_indexes_from` operates entirely through the supplied client.
#[tokio::test]
async fn check_indexes_from_uses_the_supplied_client() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");

    let owner = oximod::OxiClient::new(mongodb_uri).await?;
    let client = owner
        .client()
        .expect("OxiClient::new initializes its client");

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_explicit_client")]
    pub struct ExplicitClientCheck {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "ecc_email_uidx")]
        email: String,
    }

    let collection = ExplicitClientCheck::get_collection_from(client)?;
    let _ = collection.drop().await;

    ExplicitClientCheck::init_indexes_from(client).await?;

    let report = ExplicitClientCheck::check_indexes_from(client).await?;

    assert!(report.is_in_sync(), "expected in-sync after establishment");
    assert_eq!(report.declared().len(), 1);

    Ok(())
}

// Run test: cargo nextest run established_option_families_report_in_sync
#[tokio::test]
async fn established_option_families_report_in_sync() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_option_families")]
    pub struct OptionFamilies {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "of_code_uidx")]
        code: String,

        #[index(sparse, order = "-1", name = "of_rank_idx")]
        rank: Option<i32>,

        #[index(expire_after_secs = 3600, name = "of_ttl_idx")]
        created_at: Option<DateTime>,

        #[index(hidden, name = "of_hidden_idx")]
        secret: String,

        #[index(version = 2, name = "of_versioned_idx")]
        data: String,
    }

    reset_collection::<OptionFamilies>().await?;
    OptionFamilies::init_indexes().await?;

    let report = OptionFamilies::check_indexes().await?;

    assert_eq!(report.declared().len(), 5);
    for status in report.declared() {
        assert!(
            matches!(status, DeclaredIndexStatus::InSync { .. }),
            "expected InSync for {:?}, got {}",
            status.expected(),
            status_name(status)
        );
    }
    assert!(report.is_in_sync());
    assert!(report.unmanaged().is_empty());

    Ok(())
}

// Run test: cargo nextest run text_index_listing_is_canonicalized_to_in_sync
#[tokio::test]
async fn text_index_listing_is_canonicalized_to_in_sync() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_text_canonical")]
    pub struct TextCanonical {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(text)]
        content: String,
    }

    reset_collection::<TextCanonical>().await?;
    TextCanonical::init_indexes().await?;

    let report = TextCanonical::check_indexes().await?;

    // The server lists the text index with internal `_fts`/`_ftsx` keys;
    // the comparison must still recognize it as the declared index.
    assert_single_in_sync(&report);

    let DeclaredIndexStatus::InSync { actual, .. } = &report.declared()[0] else {
        panic!("expected InSync");
    };
    assert_eq!(
        actual.keys.get("_fts"),
        Some(&Bson::String("text".to_string())),
        "the matched server index should carry the internal text keys"
    );

    Ok(())
}

// Run test: cargo nextest run text_index_with_weight_and_languages_reports_in_sync
#[tokio::test]
async fn text_index_with_weight_and_languages_reports_in_sync() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_text_options")]
    pub struct TextOptions {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(
            text,
            weight = 5,
            default_language = "spanish",
            language_override = "lang",
            name = "to_text_idx"
        )]
        title: String,
    }

    reset_collection::<TextOptions>().await?;
    TextOptions::init_indexes().await?;

    let report = TextOptions::check_indexes().await?;

    assert_single_in_sync(&report);

    Ok(())
}

// Run test: cargo nextest run specialized_key_families_report_in_sync_with_default_names
#[tokio::test]
async fn specialized_key_families_report_in_sync_with_default_names() -> TestResult {
    init().await?;

    // No declaration carries an explicit name, so every match proves the
    // generated default name equals the name MongoDB itself assigned.
    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_default_names")]
    pub struct DefaultNames {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(order = "1")]
        ascending: i32,

        #[index(order = "-1")]
        descending: i32,

        #[index(text)]
        content: String,

        #[index(hashed)]
        user_id: String,

        #[index(wildcard)]
        attributes: Option<mongodb::bson::Document>,

        #[index(geo_2dsphere)]
        location: Vec<f64>,

        #[index(geo_2d)]
        position: Vec<f64>,
    }

    reset_collection::<DefaultNames>().await?;
    DefaultNames::init_indexes().await?;

    let report = DefaultNames::check_indexes().await?;

    assert_eq!(report.declared().len(), 7);
    assert!(
        report.is_in_sync(),
        "expected every default-named family in sync: {:?}",
        report
            .declared()
            .iter()
            .map(status_name)
            .collect::<Vec<_>>()
    );

    let expected_names = [
        "ascending_1",
        "descending_-1",
        "content_text",
        "user_id_hashed",
        "attributes.$**_1",
        "location_2dsphere",
        "position_2d",
    ];

    for (status, expected_name) in report.declared().iter().zip(expected_names) {
        let DeclaredIndexStatus::InSync { actual, .. } = status else {
            panic!("expected InSync for `{expected_name}`");
        };
        assert_eq!(
            actual
                .options
                .as_ref()
                .and_then(|options| options.name.as_deref()),
            Some(expected_name),
            "server-assigned default name should match the expected name"
        );
    }

    Ok(())
}

// Run test: cargo nextest run flat_2d_parameters_report_in_sync
#[tokio::test]
async fn flat_2d_parameters_report_in_sync() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_2d_params")]
    pub struct FlatGeo {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(geo_2d, bits = 26, min = "-180.0", max = "180.0", name = "fg_2d_idx")]
        position: Vec<f64>,
    }

    reset_collection::<FlatGeo>().await?;
    FlatGeo::init_indexes().await?;

    let report = FlatGeo::check_indexes().await?;

    assert_single_in_sync(&report);

    Ok(())
}

// Run test: cargo nextest run case_insensitive_collation_reports_in_sync
#[tokio::test]
async fn case_insensitive_collation_reports_in_sync() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_case_insensitive")]
    pub struct CaseInsensitive {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(case_insensitive, name = "ci_email_idx")]
        email: String,
    }

    reset_collection::<CaseInsensitive>().await?;
    CaseInsensitive::init_indexes().await?;

    // The server materializes the full collation document with defaulted
    // fields; the effective comparison must not report false drift.
    let report = CaseInsensitive::check_indexes().await?;

    assert_single_in_sync(&report);

    Ok(())
}

// Run test: cargo nextest run collection_default_collation_is_inherited_without_false_drift
#[tokio::test]
async fn collection_default_collation_is_inherited_without_false_drift() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_collection_collation")]
    pub struct CollectionCollation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "cc_code_idx")]
        code: String,
    }

    let collection = CollectionCollation::get_collection()?;
    let _ = collection.drop().await;

    // Create the collection with a non-simple default collation. An index
    // created without an index-level collation inherits it.
    collection
        .client()
        .database("test")
        .create_collection("ir_check_collection_collation")
        .collation(
            Collation::builder()
                .locale("en".to_string())
                .strength(CollationStrength::Secondary)
                .build(),
        )
        .await?;

    CollectionCollation::init_indexes().await?;

    let report = CollectionCollation::check_indexes().await?;

    assert_single_in_sync(&report);

    Ok(())
}

// Run test: cargo nextest run dropped_declared_index_reports_missing
#[tokio::test]
async fn dropped_declared_index_reports_missing() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_dropped")]
    pub struct Dropped {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "dr_code_uidx")]
        code: String,

        #[index(name = "dr_rank_idx")]
        rank: i32,
    }

    reset_collection::<Dropped>().await?;
    Dropped::init_indexes().await?;

    Dropped::get_collection()?
        .drop_index("dr_code_uidx")
        .await?;

    let report = Dropped::check_indexes().await?;

    // Declaration order is preserved: the dropped declaration is Missing,
    // the surviving one InSync.
    assert_eq!(report.declared().len(), 2);
    assert!(
        matches!(report.declared()[0], DeclaredIndexStatus::Missing { .. }),
        "dropped declaration should be Missing, got {}",
        status_name(&report.declared()[0])
    );
    assert!(
        matches!(report.declared()[1], DeclaredIndexStatus::InSync { .. }),
        "surviving declaration should be InSync, got {}",
        status_name(&report.declared()[1])
    );
    assert_eq!(report.missing_count(), 1);
    assert!(report.has_drift());

    Ok(())
}

// Run test: cargo nextest run expected_name_with_wrong_key_reports_mismatched
#[tokio::test]
async fn expected_name_with_wrong_key_reports_mismatched() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_name_collision")]
    pub struct NameCollision {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "nc_idx")]
        score: i32,
    }

    reset_collection::<NameCollision>().await?;

    // A raw index occupies the declaration's name with a different key.
    NameCollision::get_document_collection()?
        .create_index(
            IndexModel::builder()
                .keys(doc! { "level": 1 })
                .options(IndexOptions::builder().name("nc_idx".to_string()).build())
                .build(),
        )
        .await?;

    let report = NameCollision::check_indexes().await?;

    assert_eq!(report.mismatched_count(), 1);
    assert_eq!(report.missing_count(), 0);
    let differences = mismatch_differences(&report.declared()[0]);
    assert!(
        differences
            .iter()
            .any(|difference| difference.starts_with("keys:")),
        "expected a keys difference: {differences:?}"
    );

    Ok(())
}

// Run test: cargo nextest run same_spec_under_custom_name_reports_mismatched
#[tokio::test]
async fn same_spec_under_custom_name_reports_mismatched() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_custom_name")]
    pub struct CustomName {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(order = "1")]
        score: i32,
    }

    reset_collection::<CustomName>().await?;

    // Same key and options, but a name OxiMod's create lifecycle would
    // conflict with (IndexOptionsConflict): must not count as InSync or
    // Missing.
    CustomName::get_document_collection()?
        .create_index(
            IndexModel::builder()
                .keys(doc! { "score": 1 })
                .options(
                    IndexOptions::builder()
                        .name("operator_named_idx".to_string())
                        .build(),
                )
                .build(),
        )
        .await?;

    let report = CustomName::check_indexes().await?;

    assert_eq!(report.mismatched_count(), 1);
    assert_eq!(report.missing_count(), 0);
    let differences = mismatch_differences(&report.declared()[0]);
    assert_eq!(
        differences,
        ["name: expected `score_1`, actual `operator_named_idx`"],
        "the only difference should be the name"
    );
    assert!(
        report.unmanaged().is_empty(),
        "the mismatch candidate must not also appear as unmanaged"
    );

    Ok(())
}

// Run test: cargo nextest run option_level_drift_reports_mismatched
#[tokio::test]
async fn option_level_drift_reports_mismatched() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_option_drift")]
    pub struct OptionDrift {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "od_unique_idx")]
        code: String,

        #[index(expire_after_secs = 3600, name = "od_ttl_idx")]
        created_at: Option<DateTime>,

        #[index(hidden, name = "od_hidden_idx")]
        secret: String,
    }

    reset_collection::<OptionDrift>().await?;

    let raw = OptionDrift::get_document_collection()?;

    // Same keys and names, but drifted options: not unique, different TTL,
    // not hidden.
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "code": 1 })
            .options(
                IndexOptions::builder()
                    .name("od_unique_idx".to_string())
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
                    .name("od_ttl_idx".to_string())
                    .expire_after(std::time::Duration::from_secs(60))
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
                    .name("od_hidden_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;

    let report = OptionDrift::check_indexes().await?;

    assert_eq!(report.mismatched_count(), 3);
    assert_eq!(report.missing_count(), 0);

    let unique_differences = mismatch_differences(&report.declared()[0]);
    assert_eq!(unique_differences, ["unique: expected true, actual false"]);

    let ttl_differences = mismatch_differences(&report.declared()[1]);
    assert_eq!(
        ttl_differences,
        ["expire_after_secs: expected 3600, actual 60"]
    );

    let hidden_differences = mismatch_differences(&report.declared()[2]);
    assert_eq!(hidden_differences, ["hidden: expected true, actual false"]);

    Ok(())
}

// Run test: cargo nextest run raw_partial_index_with_same_key_reports_mismatched
#[tokio::test]
async fn raw_partial_index_with_same_key_reports_mismatched() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_partial_collision")]
    pub struct PartialCollision {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(order = "1")]
        rating: i32,
    }

    reset_collection::<PartialCollision>().await?;

    // A raw partial index occupies the same key family and default name.
    PartialCollision::get_document_collection()?
        .create_index(
            IndexModel::builder()
                .keys(doc! { "rating": 1 })
                .options(
                    IndexOptions::builder()
                        .partial_filter_expression(doc! { "rating": { "$gt": 5 } })
                        .build(),
                )
                .build(),
        )
        .await?;

    let report = PartialCollision::check_indexes().await?;

    assert_eq!(report.mismatched_count(), 1);
    assert_eq!(report.missing_count(), 0);
    let differences = mismatch_differences(&report.declared()[0]);
    assert!(
        differences
            .iter()
            .any(|difference| difference.starts_with("partial_filter_expression:")),
        "expected a partial filter difference: {differences:?}"
    );

    Ok(())
}

// Run test: cargo nextest run unmanaged_raw_indexes_do_not_break_sync
#[tokio::test]
async fn unmanaged_raw_indexes_do_not_break_sync() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_unmanaged")]
    pub struct Unmanaged {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(unique, name = "um_code_uidx")]
        code: String,
    }

    reset_collection::<Unmanaged>().await?;
    Unmanaged::init_indexes().await?;

    // Direct-driver escape hatches: a compound index and a partial index on
    // an unrelated field.
    let raw = Unmanaged::get_document_collection()?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "region": 1, "city": -1 })
            .options(
                IndexOptions::builder()
                    .name("um_compound_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "score": 1 })
            .options(
                IndexOptions::builder()
                    .name("um_partial_idx".to_string())
                    .partial_filter_expression(doc! { "score": { "$gt": 0 } })
                    .build(),
            )
            .build(),
    )
    .await?;

    let report = Unmanaged::check_indexes().await?;

    // Unmanaged indexes are surfaced deterministically but never count as
    // drift, and `_id_` is never reported.
    assert!(report.is_in_sync());
    let unmanaged_names: Vec<_> = report
        .unmanaged()
        .iter()
        .map(|index| {
            index
                .options
                .as_ref()
                .and_then(|options| options.name.as_deref())
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(unmanaged_names, ["um_compound_idx", "um_partial_idx"]);

    Ok(())
}

// Run test: cargo nextest run id_index_is_ignored_entirely
#[tokio::test]
async fn id_index_is_ignored_entirely() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_id_ignored")]
    pub struct IdIgnored {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        note: String,
    }

    reset_collection::<IdIgnored>().await?;

    // Materialize the collection (and its `_id_` index) without any OxiMod
    // index machinery.
    IdIgnored::get_document_collection()?
        .insert_one(doc! { "note": "n" })
        .await?;

    let report = IdIgnored::check_indexes().await?;

    assert!(report.declared().is_empty());
    assert!(
        report.unmanaged().is_empty(),
        "`_id_` must not be reported as unmanaged"
    );
    assert!(report.is_in_sync());

    Ok(())
}

// Run test: cargo nextest run raw_text_index_on_other_field_blocks_text_declaration
#[tokio::test]
async fn raw_text_index_on_other_field_blocks_text_declaration() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_text_conflict")]
    pub struct TextConflict {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(text)]
        content: String,
    }

    reset_collection::<TextConflict>().await?;

    // MongoDB allows at most one text index per collection, so a raw text
    // index on another field makes the declaration Mismatched, never
    // Missing (creation would fail).
    TextConflict::get_document_collection()?
        .create_index(
            IndexModel::builder()
                .keys(doc! { "summary": "text" })
                .build(),
        )
        .await?;

    let report = TextConflict::check_indexes().await?;

    assert_eq!(report.mismatched_count(), 1);
    assert_eq!(report.missing_count(), 0);
    let differences = mismatch_differences(&report.declared()[0]);
    assert!(
        differences
            .iter()
            .any(|difference| difference.starts_with("text fields:")),
        "expected a text fields difference: {differences:?}"
    );

    Ok(())
}

// Run test: cargo nextest run intrinsically_sparse_geo_declarations_ignore_missing_sparse_flag
#[tokio::test]
async fn intrinsically_sparse_geo_declarations_ignore_missing_sparse_flag() -> TestResult {
    init().await?;

    // 2dsphere (v2+) and 2d indexes are always sparse and ignore the
    // `sparse` creation option, so a declared flag must not drift against
    // equivalent raw indexes created without it.
    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_geo_sparse")]
    pub struct GeoSparse {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(geo_2dsphere, sparse, name = "gs_sphere_idx")]
        location: Vec<f64>,

        #[index(geo_2d, sparse, name = "gs_flat_idx")]
        position: Vec<f64>,
    }

    reset_collection::<GeoSparse>().await?;

    let raw = GeoSparse::get_document_collection()?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "location": "2dsphere" })
            .options(
                IndexOptions::builder()
                    .name("gs_sphere_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "position": "2d" })
            .options(
                IndexOptions::builder()
                    .name("gs_flat_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;

    let report = GeoSparse::check_indexes().await?;

    assert_eq!(report.declared().len(), 2);
    assert!(
        report.is_in_sync(),
        "intrinsically sparse geo indexes must not drift on the sparse \
         flag: {:?}",
        report
            .declared()
            .iter()
            .map(status_name)
            .collect::<Vec<_>>()
    );

    Ok(())
}

// Run test: cargo nextest run wildcard_sparse_is_rejected_by_the_server_and_is_not_drift
#[tokio::test]
async fn wildcard_sparse_is_rejected_by_the_server_and_is_not_drift() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_wildcard_sparse")]
    pub struct WildcardSparse {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(wildcard, sparse, name = "ws_attrs_idx")]
        attributes: Option<mongodb::bson::Document>,
    }

    reset_collection::<WildcardSparse>().await?;

    let raw = WildcardSparse::get_document_collection()?;

    // Pin the observed server behavior: wildcard indexes are inherently
    // sparse, and MongoDB rejects the explicit option outright, so no
    // actual wildcard listing can ever carry it.
    let rejection = raw
        .create_index(
            IndexModel::builder()
                .keys(doc! { "attributes.$**": 1 })
                .options(
                    IndexOptions::builder()
                        .name("ws_probe_idx".to_string())
                        .sparse(true)
                        .build(),
                )
                .build(),
        )
        .await
        .expect_err("MongoDB should reject `sparse` on a wildcard index");
    assert!(
        format!("{rejection:?}").contains("CannotCreateIndex"),
        "expected CannotCreateIndex (code 67), got {rejection:?}"
    );

    // The declared flag therefore has no semantic content: an equivalent
    // wildcard index created without it satisfies the declaration.
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "attributes.$**": 1 })
            .options(
                IndexOptions::builder()
                    .name("ws_attrs_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;

    let report = WildcardSparse::check_indexes().await?;

    assert!(
        report.is_in_sync(),
        "wildcard declarations must not drift on the sparse flag: {:?}",
        report
            .declared()
            .iter()
            .map(status_name)
            .collect::<Vec<_>>()
    );

    Ok(())
}

// Run test: cargo nextest run locale_specific_collation_defaults_do_not_create_false_drift
#[tokio::test]
async fn locale_specific_collation_defaults_do_not_create_false_drift() -> TestResult {
    init().await?;

    // `fr_CA` carries a locale-specific collation default: omitted
    // `backwards` resolves to true, unlike the generic default of false.
    // Verified against MongoDB 8.0: both listCollections (the collection
    // default) and listIndexes (the inherited index collation) materialize
    // the full effective collation document — locale defaults included — so
    // the effective comparison sees identical resolved values on both
    // sides.
    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_locale_collation")]
    pub struct LocaleCollation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "lc_title_idx")]
        title: String,
    }

    let collection = LocaleCollation::get_collection()?;
    let _ = collection.drop().await;

    collection
        .client()
        .database("test")
        .create_collection("ir_check_locale_collation")
        .collation(Collation::builder().locale("fr_CA".to_string()).build())
        .await?;

    LocaleCollation::init_indexes().await?;

    let report = LocaleCollation::check_indexes().await?;

    assert_single_in_sync(&report);

    Ok(())
}

// Run test: cargo nextest run explicit_deviation_from_locale_default_reports_mismatched
#[tokio::test]
async fn explicit_deviation_from_locale_default_reports_mismatched() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize)]
    #[db("test")]
    #[collection("ir_check_locale_deviation")]
    pub struct LocaleDeviation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[index(name = "ld_title_idx")]
        title: String,
    }

    let collection = LocaleDeviation::get_collection()?;
    let _ = collection.drop().await;

    collection
        .client()
        .database("test")
        .create_collection("ir_check_locale_deviation")
        .collation(Collation::builder().locale("fr_CA".to_string()).build())
        .await?;

    // An otherwise matching raw index explicitly overrides `backwards` to
    // false — deviating from the collection's effective `fr_CA` default of
    // true. The declaration expects the inherited default, so this is real
    // drift.
    let mut deviating = Collation::builder().locale("fr_CA".to_string()).build();
    deviating.backwards = Some(false);

    LocaleDeviation::get_document_collection()?
        .create_index(
            IndexModel::builder()
                .keys(doc! { "title": 1 })
                .options(
                    IndexOptions::builder()
                        .name("ld_title_idx".to_string())
                        .collation(deviating)
                        .build(),
                )
                .build(),
        )
        .await?;

    let report = LocaleDeviation::check_indexes().await?;

    assert_eq!(report.mismatched_count(), 1);
    let differences = mismatch_differences(&report.declared()[0]);
    assert_eq!(differences.len(), 1);
    assert!(
        differences[0].starts_with("collation:")
            && differences[0].contains("backwards: true")
            && differences[0].contains("backwards: false"),
        "expected a backwards collation difference: {differences:?}"
    );

    Ok(())
}
