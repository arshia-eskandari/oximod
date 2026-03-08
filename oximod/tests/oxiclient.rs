use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

// Run test: cargo nextest run oxiclient_works
#[tokio::test]
async fn oxiclient_works() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("oxiclient_works")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");

    let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    let client = oxiclient.client().unwrap();

    User::clear_from(client).await?;
    let id = ObjectId::new();
    let user_new = User::new().id(id);
    user_new.save_from(client).await?;

    let collection = User::get_collection_from(client)?;
    let result = collection.find_one(doc! { "_id": id }).await?;
    assert!(result.is_some());
    Ok(())
}
