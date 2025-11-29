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
    let user = User::default()
        .id(id.clone())
        .name("User1".to_string())
        .age(31)
        .active(true);

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
    let user = User::default()
        .id(id.clone())
        .name("User1".to_string())
        .age(31)
        .active(true);

    user.save().await?;

    // Invalid update: $set is scalar
    let result = User::update_by_id(id, doc! { "$set": "invalid" }).await;

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
    let user = User::default()
        .id(id.clone())
        .name("User1".to_string())
        .age(31)
        .active(true);

    user.save().await?;

    // Update email to valid email
    let result =
        User::update_by_id(id.clone(), doc! { "$set": { "email": "user@example.com" } }).await?;

    assert_eq!(result.matched_count, 1);
    assert_eq!(result.modified_count, 1);

    let updated = User::find_by_id(id).await?;
    assert!(updated.is_some());

    if let Some(u) = updated {
        assert_eq!(u.email.as_deref(), Some("user@example.com"));
    }

    Ok(())
}
