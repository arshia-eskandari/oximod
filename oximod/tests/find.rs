use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run finds_multiple_matching_documents
#[tokio::test]
async fn finds_multiple_matching_documents() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1".to_string()).age(28).active(true),
        User::default().name("User2".to_string()).age(28).active(true),
        User::default().name("User3".to_string()).age(35).active(true)
    ];

    for user in users {
        user.save().await?;
    }

    let matched_users = User::find(doc! { "age": 28 }).await?;
    assert_eq!(matched_users.len(), 2);

    let names: Vec<String> = matched_users
        .into_iter()
        .map(|u| u.name)
        .collect();
    assert!(names.contains(&"User1".to_string()));
    assert!(names.contains(&"User2".to_string()));

    Ok(())
}
