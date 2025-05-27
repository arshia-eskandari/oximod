use futures_util::stream::StreamExt;
use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use serde::{ Deserialize, Serialize };
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run aggregates_documents_correctly
#[tokio::test]
async fn aggregates_documents_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("aggregate_test")]
    pub struct LogEntry {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        level: String,
        message: String,
    }

    LogEntry::clear().await?;

    let logs = vec![
        LogEntry::default().level("INFO".to_string()).message("Startup complete".to_string()),
        LogEntry::default().level("ERROR".to_string()).message("Failed to connect".to_string()),
        LogEntry::default().level("INFO".to_string()).message("Listening on port 3000".to_string())
    ];

    for log in logs {
        log.save().await?;
    }

    let pipeline = vec![doc! { "$match": { "level": "INFO" } }, doc! { "$count": "info_count" }];

    let mut cursor = LogEntry::aggregate(pipeline).await?;
    let result = cursor.next().await.expect("Expected at least one document")?;
    let count = result.get_i32("info_count").expect("Expected 'info_count' field");

    assert_eq!(count, 2);

    Ok(())
}
