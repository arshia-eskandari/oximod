use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run updates_document_by_id_correctly
#[tokio::test]
async fn updates_document_by_id_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("ptest")]
    #[collection("update_by_id")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let id = ObjectId::new();
    let user = User::default().id(id.clone()).name("User1".to_string()).age(31).active(true);

    user.save().await?;

    // Update age to 32
    User::update_by_id(id, doc! { "$set": { "age": 32 } }).await?;

    let updated = User::find_by_id(id).await?;
    assert!(updated.is_some());

    if let Some(u) = updated {
        assert_eq!(u.age, 32);
        assert_eq!(u.name, "User1");
    }

    Ok(())
}
