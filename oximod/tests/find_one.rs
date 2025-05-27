use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run finds_first_matching_document_correctly
#[tokio::test]
async fn finds_first_matching_document_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_one")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1".to_string()).age(22).active(true),
        User::default().name("User2".to_string()).age(22).active(false)
    ];

    for user in users {
        user.save().await?;
    }

    let matched = User::find_one(doc! { "age": 22 }).await?;
    assert!(matched.is_some());

    if let Some(user) = matched {
        assert_eq!(user.age, 22);
        assert!(["User1", "User2"].contains(&user.name.as_str()));
    }

    Ok(())
}
