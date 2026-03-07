use futures_util::stream::StreamExt;
use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run aggregates_documents_correctly
#[tokio::test]
async fn aggregates_documents_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("logs_for_aggregates_documents_correctly")]
    pub struct LogEntry {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        level: String,
        message: String,
    }

    LogEntry::clear().await?;

    let logs = vec![
        LogEntry::default()
            .level("INFO")
            .message("Startup complete"),
        LogEntry::default()
            .level("ERROR")
            .message("Failed to connect"),
        LogEntry::default()
            .level("INFO")
            .message("Listening on port 3000"),
    ];

    for log in logs {
        log.save().await?;
    }

    let pipeline = vec![
        doc! { "$match": { "level": "INFO" } },
        doc! { "$count": "info_count" },
    ];

    let log_collection = LogEntry::get_document_collection()?;
    let mut cursor = log_collection.aggregate(pipeline).await?;
    let result = cursor
        .next()
        .await
        .expect("Expected at least one document")?;
    let count = result
        .get_i32("info_count")
        .expect("Expected 'info_count' field");

    assert_eq!(count, 2);

    Ok(())
}

// Run test: cargo nextest run aggregation_with_no_matches_returns_empty
#[tokio::test]
async fn aggregation_with_no_matches_returns_empty() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("logs_for_aggregation_no_matches")]
    pub struct LogEntry {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        level: String,
        message: String,
    }

    LogEntry::clear().await?;

    let logs = vec![
        LogEntry::default()
            .level("INFO")
            .message("Startup complete"),
        LogEntry::default()
            .level("ERROR")
            .message("Failed to connect"),
    ];

    for log in logs {
        log.save().await?;
    }

    // Query for a level that doesn't exist
    let pipeline = vec![
        doc! { "$match": { "level": "DEBUG" } },
        doc! { "$count": "debug_count" },
    ];

    let log_collection = LogEntry::get_document_collection()?;
    let mut cursor = log_collection.aggregate(pipeline).await?;
    assert!(
        cursor.next().await.is_none(),
        "Expected no documents in aggregation result"
    );

    Ok(())
}

// Run test: cargo nextest run aggregates_count_emails_ending_with_com_including_missing_emails
#[tokio::test]
async fn aggregates_count_emails_ending_with_com_including_missing_emails() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("logs_for_aggregates_count_emails_ending_with_com")]
    pub struct LogEntry {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        level: String,
        message: String,

        #[validate(email)]
        email: Option<String>,
    }

    LogEntry::clear().await?;

    let logs = vec![
        LogEntry::default()
            .level("INFO")
            .message("Startup complete")
            .email("info1@example.com"),
        LogEntry::default()
            .level("INFO")
            .message("Listening on port 3000")
            .email("info2@example.org"),
        LogEntry::default()
            .level("ERROR")
            .message("Failed to connect")
            .email("error@example.com"),
        LogEntry::default()
            .level("WARN")
            .message("No email set for this log"), // email == None
        LogEntry::default()
            .level("DEBUG")
            .message("Another log with no email"), // email == None
    ];

    for log in logs {
        log.save().await?;
    }

    // Aggregate: count documents with emails ending in `.com`
    let pipeline = vec![
        doc! { "$match": { "email": { "$regex": "\\.com$" } } },
        doc! { "$count": "com_count" },
    ];

    let log_collection = LogEntry::get_document_collection()?;
    let mut cursor = log_collection.aggregate(pipeline).await?;
    let result = cursor
        .next()
        .await
        .expect("Expected at least one document")?;
    let count = result
        .get_i32("com_count")
        .expect("Expected 'com_count' field");

    assert_eq!(count, 2);

    Ok(())
}
