use mongodb::bson::{doc, oid::ObjectId};
use oximod::{set_global_client, Model};
use testresult::TestResult;
use serde::{Deserialize, Serialize};

// Run test: cargo nextest run updates_multiple_documents_correctly
#[tokio::test]
async fn updates_multiple_documents_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

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
        User {
            _id: None,
            name: "User1".to_string(),
            age: 70,
            active: true,
        },
        User {
            _id: None,
            name: "User2".to_string(),
            age: 65,
            active: true,
        },
        User {
            _id: None,
            name: "User3".to_string(),
            age: 40,
            active: true,
        },
    ];

    for user in users {
        user.save().await?;
    }

    // Deactivate users aged 65+
    let result = User::update(
        doc! { "age": { "$gte": 65 } },
        doc! { "$set": { "active": false } },
    )
    .await?;

    assert_eq!(result.matched_count, 2);
    assert_eq!(result.modified_count, 2);

    Ok(())
}
