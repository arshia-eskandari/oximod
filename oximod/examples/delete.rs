//! Delete documents safely with OxiMod's typed-query API.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example delete
//! ```
//!
//! This example demonstrates how to:
//!
//! - build type-safe deletion filters;
//! - use sorting to choose one matching document;
//! - receive the deleted model from `delete_one()`;
//! - delete every document matching a typed filter;
//! - inspect MongoDB's deleted count;
//! - handle a deletion that finds no matching document.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiClient, Queryable};
use serde::{Deserialize, Serialize};

// A login session stored as an independent MongoDB document.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("delete_sessions")]
struct LoginSession {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    user_name: String,
    device: String,

    #[validate(non_negative)]
    inactive_days: i32,

    #[default(false)]
    compromised: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // This collection belongs only to the example, so clearing it keeps
    // repeated runs deterministic.
    LoginSession::clear().await?;

    seed_sessions().await?;

    let initial_count = LoginSession::query().count().await?;

    println!("Sessions before cleanup: {initial_count}");

    // Two compromised sessions match. Sorting makes the selection explicit:
    // the session inactive for the longest time is deleted first.
    //
    // `delete_one()` returns the deleted model rather than only a count.
    let deleted_compromised_session = LoginSession::query()
        .filter(|session| session.compromised.eq(true))
        .sort_by(|session| session.inactive_days.desc())
        .delete_one()
        .await?;

    match deleted_compromised_session {
        Some(session) => {
            println!(
                "Deleted compromised session: {} on {} \
                 (inactive for {} days)",
                session.user_name, session.device, session.inactive_days,
            );
        }
        None => {
            println!("No compromised session was found");
        }
    }

    // Bulk typed deletion accepts filters but not sorting, skipping, limiting,
    // or pagination. Every remaining session inactive for at least 30 days is
    // removed.
    let stale_deletion = LoginSession::query()
        .filter(|session| session.inactive_days.gte(30))
        .delete_all()
        .await?;

    println!(
        "Deleted {} additional stale sessions",
        stale_deletion.deleted_count
    );

    // A missing match is not an error. `delete_one()` returns `None`.
    let missing_deletion = LoginSession::query()
        .filter(|session| session.user_name.eq("MissingUser"))
        .delete_one()
        .await?;

    println!(
        "Missing-session deletion found a document: {}",
        missing_deletion.is_some()
    );

    let remaining_sessions = LoginSession::query()
        .sort_by(|session| session.user_name.asc())
        .all()
        .await?;

    println!("Remaining sessions: {}", remaining_sessions.len());

    for session in remaining_sessions {
        println!(
            "  {} on {} (inactive for {} days)",
            session.user_name, session.device, session.inactive_days,
        );
    }

    Ok(())
}

/// Inserts a deterministic set of sessions for the cleanup workflow.
async fn seed_sessions() -> Result<(), oximod::OxiModError> {
    let sessions = [
        LoginSession::new()
            .user_name("User1")
            .device("laptop")
            .inactive_days(2),
        LoginSession::new()
            .user_name("User2")
            .device("tablet")
            .inactive_days(45),
        LoginSession::new()
            .user_name("User3")
            .device("phone")
            .inactive_days(90)
            .compromised(true),
        LoginSession::new()
            .user_name("User4")
            .device("desktop")
            .inactive_days(60)
            .compromised(true),
        LoginSession::new()
            .user_name("User5")
            .device("phone")
            .inactive_days(10),
    ];

    for session in sessions {
        session.save().await?;
    }

    Ok(())
}
