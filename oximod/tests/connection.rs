//! Integration test for global MongoDB client initialization.
//!
//! The test verifies that shared setup establishes a client retrievable
//! through `OxiClient::global()`.

use oximod::OxiClient;
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run connects_to_db_successfully
#[tokio::test]
async fn connects_to_db_successfully() -> TestResult {
    init().await?;

    OxiClient::global().unwrap_or_else(|e| panic!("{}", e));

    Ok(())
}
