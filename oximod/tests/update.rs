use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run updates_multiple_documents_correctly
#[tokio::test]
async fn updates_multiple_documents_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let users = vec![
        User::default().name("User1".to_string()).age(70).active(true),
        User::default().name("User2".to_string()).age(65).active(true),
        User::default().name("User3".to_string()).age(40).active(true)
    ];

    for user in users {
        user.save().await?;
    }

    // Deactivate users aged 65+
    let result = User::update(
        doc! { "age": { "$gte": 65 } },
        doc! { "$set": { "active": false } }
    ).await?;

    assert_eq!(result.matched_count, 2);
    assert_eq!(result.modified_count, 2);

    Ok(())
}
