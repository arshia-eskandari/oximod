//! Detect index drift and recreate genuinely missing declared indexes.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example index_reconciliation
//! ```
//!
//! This example demonstrates how to:
//!
//! - establish declared `#[index(...)]` specifications at startup;
//! - simulate operational drift by dropping a declared index and adding a
//!   conflicting variant of another out-of-band;
//! - inspect drift with the read-only `check_indexes_from()`;
//! - repair only the genuinely missing declaration with
//!   `create_missing_indexes_from()`;
//! - observe that mismatched and unmanaged raw-driver indexes are reported
//!   but never modified.
//!
//! Creating a missing index is still operationally consequential: index
//! builds consume resources, a unique index build fails when existing data
//! contains duplicates, and a new TTL index can immediately expire old
//! documents. Run
//! reconciliation during controlled startup, deployment, or maintenance
//! workflows.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::{
    IndexModel,
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
};
use oximod::{DeclaredIndexStatus, Model, OxiClient};
use serde::{Deserialize, Serialize};

// A user profile with two declared single-field indexes.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("reconciliation_users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(unique, name = "email_uidx")]
    email: String,

    #[index(order = "-1", name = "signup_rank_idx")]
    signup_rank: i32,
}

fn describe(report: &oximod::IndexDriftReport) {
    for status in report.declared() {
        match status {
            DeclaredIndexStatus::InSync { expected, .. } => {
                println!("  in sync:    {:?}", expected.keys);
            }
            DeclaredIndexStatus::Missing { expected } => {
                println!("  missing:    {:?}", expected.keys);
            }
            DeclaredIndexStatus::Mismatched {
                expected,
                candidates,
            } => {
                println!("  mismatched: {:?}", expected.keys);
                for candidate in candidates {
                    for difference in candidate.differences() {
                        println!("      - {difference}");
                    }
                }
            }
            other => println!("  other:      {other:?}"),
        }
    }
    for unmanaged in report.unmanaged() {
        println!("  unmanaged:  {:?} (left untouched)", unmanaged.keys);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri =
        std::env::var("MONGODB_URI").expect("Set MONGODB_URI in your environment or .env file");

    let owner = OxiClient::new(mongodb_uri).await?;
    let client = owner
        .client()
        .expect("OxiClient::new initializes its client");

    // Start from a clean collection, then establish the declared indexes.
    let collection = User::get_collection_from(client)?;
    let _ = collection.drop().await;
    User::init_indexes_from(client).await?;

    println!("after startup establishment:");
    let report = User::check_indexes_from(client).await?;
    describe(&report);
    assert!(report.is_in_sync());

    // Simulate operational drift out-of-band, as a migration script or a
    // teammate with a mongo shell might:
    // 1. the unique email index disappears;
    // 2. the rank index is replaced by a non-declared variant under the
    //    declared name (ascending instead of descending);
    // 3. an unmanaged raw compound index exists for application needs.
    let raw = User::get_document_collection_from(client)?;
    collection.drop_index("email_uidx").await?;
    collection.drop_index("signup_rank_idx").await?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "signup_rank": 1 })
            .options(
                IndexOptions::builder()
                    .name("signup_rank_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;
    raw.create_index(
        IndexModel::builder()
            .keys(doc! { "region": 1, "email": 1 })
            .options(
                IndexOptions::builder()
                    .name("region_email_idx".to_string())
                    .build(),
            )
            .build(),
    )
    .await?;

    println!("\nafter out-of-band changes:");
    let drift = User::check_indexes_from(client).await?;
    describe(&drift);
    assert!(drift.has_drift());
    assert_eq!(drift.missing_count(), 1);
    assert_eq!(drift.mismatched_count(), 1);

    // Conservative repair: only the genuinely missing declaration is
    // created. The mismatched index and the unmanaged compound index are
    // reported, never dropped or modified.
    let result = User::create_missing_indexes_from(client).await?;

    println!(
        "\nreconciliation submitted {} declaration(s):",
        result.attempted_creations().len()
    );
    describe(result.after());

    assert_eq!(result.attempted_creations().len(), 1);
    assert_eq!(result.after().missing_count(), 0);
    assert_eq!(result.after().mismatched_count(), 1);

    if !result.is_in_sync() {
        println!(
            "\nremaining mismatches require a human decision; OxiMod never \
             drops or rewrites an existing index"
        );
    }

    Ok(())
}
