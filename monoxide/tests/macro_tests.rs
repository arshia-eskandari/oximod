use monoxide_core::feature::conn::client::set_global_client;
use monoxide_macros::Model;
use testresult::TestResult;
use monoxide_core::feature::model::Model;

#[tokio::test]
async fn saves_document_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model)]
    pub struct User {
        name: String,
        age: i32,
        active: bool,
    }

    let user = User {
        name: "Arshia".to_string(),
        age: 25,
        active: true,
    };

    user.save().await?;

    Ok(())
}
