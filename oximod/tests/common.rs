use oximod::set_global_client;

pub async fn init() {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
    set_global_client(mongodb_uri).await.unwrap();
}
