use mongodb::bson::{doc, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod common;
use common::init;

// Run test: cargo nextest run updates_document_by_id_correctly
#[tokio::test]
async fn updates_document_by_id_correctly() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
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
    let user = User::default().id(id).name("User1").age(31).active(true);

    user.save().await?;

    let collection = User::get_collection()?;

    // Update age to 32
    collection
        .update_one(doc! { "_id": id }, doc! { "$set": { "age": 32 } })
        .await?;

    let updated = collection.find_one(doc! { "_id": id }).await?;
    assert!(updated.is_some());

    if let Some(u) = updated {
        assert_eq!(u.age, 32);
        assert_eq!(u.name, "User1");
    }

    Ok(())
}

// Run test: cargo nextest run updates_document_by_id_invalid_update_fails
#[tokio::test]
async fn updates_document_by_id_invalid_update_fails() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update_by_id_invalid")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let id = ObjectId::new();
    let user = User::default().id(id).name("User1").age(31).active(true);

    user.save().await?;

    let collection = User::get_collection()?;

    // Invalid update
    let result = collection
        .update_one(doc! { "_id": id }, doc! { "$set": "invalid" })
        .await;

    assert!(result.is_err());

    Ok(())
}

// Run test: cargo nextest run updates_by_id_optional_email_to_valid
#[tokio::test]
async fn updates_by_id_optional_email_to_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update_by_id_optional_email_valid")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        age: i32,
        active: bool,

        #[validate(email)]
        email: Option<String>,
    }

    User::clear().await?;

    let id = ObjectId::new();
    let user = User::default().id(id).name("User1").age(31).active(true);

    user.save().await?;

    let collection = User::get_collection()?;

    let result = collection
        .update_one(
            doc! { "_id": id },
            doc! { "$set": { "email": "user@example.com" } },
        )
        .await?;

    assert_eq!(result.matched_count, 1);
    assert_eq!(result.modified_count, 1);

    let updated = collection.find_one(doc! { "_id": id }).await?;
    assert!(updated.is_some());

    if let Some(u) = updated {
        assert_eq!(u.email.as_deref(), Some("user@example.com"));
    }

    Ok(())
}

// Run test: cargo nextest run updates_document_by_id_correctly_using_model_helper
#[tokio::test]
async fn updates_document_by_id_correctly_using_model_helper() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update_by_id_test_correctly_using_model_helper")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let id = ObjectId::new();
    let user = User::default().id(id).name("User1").age(31).active(true);

    user.save().await?;

    let result = User::update_by_id(id, doc! { "$set": { "age": 32 } }).await?;
    assert_eq!(result.matched_count, 1);
    assert_eq!(result.modified_count, 1);

    let collection = User::get_collection()?;
    let updated = collection.find_one(doc! { "_id": id }).await?;
    assert!(updated.is_some());

    if let Some(u) = updated {
        assert_eq!(u.age, 32);
        assert_eq!(u.name, "User1");
    }

    Ok(())
}

// Run test: cargo nextest run updates_document_by_id_invalid_update_fails_using_model_helper
#[tokio::test]
async fn updates_document_by_id_invalid_update_fails_using_model_helper() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update_by_id_test_invalid_update_fails_using_model_helper")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let id = ObjectId::new();
    let user = User::default().id(id).name("User1").age(31).active(true);

    user.save().await?;

    let result = User::update_by_id(id, doc! { "$set": "invalid" }).await;
    assert!(result.is_err());

    Ok(())
}

// Run test: cargo nextest run updates_by_id_optional_email_to_valid_using_model_helper
#[tokio::test]
async fn updates_by_id_optional_email_to_valid_using_model_helper() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("update_by_id_test_optional_email_valid_using_model_helper")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        age: i32,
        active: bool,

        #[validate(email)]
        email: Option<String>,
    }

    User::clear().await?;

    let id = ObjectId::new();
    let user = User::default().id(id).name("User1").age(31).active(true);

    user.save().await?;

    let result = User::update_by_id(id, doc! { "$set": { "email": "user@example.com" } }).await?;

    assert_eq!(result.matched_count, 1);
    assert_eq!(result.modified_count, 1);

    let collection = User::get_collection()?;
    let updated = collection.find_one(doc! { "_id": id }).await?;
    assert!(updated.is_some());

    if let Some(u) = updated {
        assert_eq!(u.email.as_deref(), Some("user@example.com"));
    }

    Ok(())
}
