use mongodb::bson::oid::ObjectId;
use monoxide_core::feature::conn::client::set_global_client;
use monoxide_macros::Model;
use testresult::TestResult;
use monoxide_core::feature::model::Model;
use serde::{ Deserialize, Serialize };

#[tokio::test]
async fn saves_document_without_id_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize)]
    #[db("db_name")]
    #[collection("collection_name")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    let user = User {
        _id: None,
        name: "Arshia".to_string(),
        age: 25,
        active: true,
    };

    user.save().await?;

    Ok(())
}

#[tokio::test]
async fn saves_document_with_id_correctly() -> TestResult {
    use mongodb::bson::oid::ObjectId;

    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize)]
    #[db("db_name")]
    #[collection("collection_name")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    let user = User {
        _id: Some(ObjectId::new()), // explicitly set _id
        name: "Ali".to_string(),
        age: 30,
        active: false,
    };

    user.save().await?;

    Ok(())
}

#[tokio::test]
async fn deactivates_senior_users_correctly() -> TestResult {
    use mongodb::bson::{doc, oid::ObjectId};
    use monoxide_macros::Model;

    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("db_name")]
    #[collection("collection_name")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    let users = vec![
        User {
            _id: None,
            name: "James".to_string(),
            age: 70,
            active: true,
        },
        User {
            _id: None,
            name: "Ali".to_string(),
            age: 65,
            active: true,
        },
        User {
            _id: None,
            name: "Sophia".to_string(),
            age: 40,
            active: true,
        },
    ];

    for user in users {
        user.save().await?;
    }

    // Deactivate users aged 65+
    User::update(
        doc! { "age": { "$gte": 65 } },
        doc! { "$set": { "active": false } },
    )
    .await?;

    Ok(())
}