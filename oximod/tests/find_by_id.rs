use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run finds_document_by_id_correctly
#[tokio::test]
async fn finds_document_by_id_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_by_id")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let id = ObjectId::new();
    let user = User::default().id(id.clone()).name("User1".to_string()).age(33).active(true);

    user.save().await?;

    let found = User::find_by_id(id).await?;
    assert!(found.is_some());

    if let Some(u) = found {
        assert_eq!(u._id, Some(id));
        assert_eq!(u.name, "User1");
        assert_eq!(u.age, 33);
        assert!(u.active);
    }

    Ok(())
}
