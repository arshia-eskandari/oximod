use monoxide_core::feature::conn::client::{get_global_client, set_global_client};
use testresult::TestResult;

// Run test: cargo nextest run connects_to_db_successfully
#[tokio::test]
async fn connects_to_db_successfully() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    get_global_client().unwrap_or_else(|e| panic!("{}", e));

    Ok(())
}
