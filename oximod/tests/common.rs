use oximod::OxiClient;
use oximod_core::error::oximod_error::OxiModError;

pub async fn init() -> Result<(), OxiModError> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
    OxiClient::init_global(mongodb_uri).await?;
    Ok(())
}
