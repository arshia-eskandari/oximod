use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run checks_existence_of_matching_document
#[tokio::test]
async fn checks_existence_of_matching_document() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("exists")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let user = User::default().name("User1".to_string()).age(27).active(true);

    user.save().await?;

    let exists = User::exists(doc! { "name": "User1" }).await?;
    assert!(exists);

    let not_exists = User::exists(doc! { "name": "SomeoneWhoDoesNotExist" }).await?;
    assert!(!not_exists);

    Ok(())
}
