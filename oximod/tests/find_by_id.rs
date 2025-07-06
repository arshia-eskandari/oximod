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
    #[collection("find_by_id_test_finds_document_by_id_correctly")]
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

// Run test: cargo nextest run finds_document_by_id_returns_none_when_not_found
#[tokio::test]
async fn finds_document_by_id_returns_none_when_not_found() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_by_id_test_returns_none_when_not_found")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    let id_that_does_not_exist = ObjectId::new();

    let found = User::find_by_id(id_that_does_not_exist).await?;
    assert!(found.is_none(), "Expected no document to be found for non-existent _id");

    Ok(())
}

// Run test: cargo nextest run finds_document_by_id_with_email_correctly
#[tokio::test]
async fn finds_document_by_id_with_email_correctly() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_by_id_test_with_email_correctly")]
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
        .age(33)
        .active(true)
        .email("user1@example.com".to_string()); // setter takes String

    user.save().await?;

    let found = User::find_by_id(id).await?;
    assert!(found.is_some());

    if let Some(u) = found {
        assert_eq!(u._id, Some(id));
        assert_eq!(u.name, "User1");
        assert_eq!(u.age, 33);
        assert!(u.active);
        assert_eq!(u.email.as_deref(), Some("user1@example.com"));
    }

    Ok(())
}

// Run test: cargo nextest run finds_document_by_id_returns_none_when_not_found_with_email
#[tokio::test]
async fn finds_document_by_id_returns_none_when_not_found_with_email() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("find_by_id_test_returns_none_when_not_found_with_email")]
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

    let id_that_does_not_exist = ObjectId::new();

    let found = User::find_by_id(id_that_does_not_exist).await?;
    assert!(found.is_none(), "Expected no document to be found for non-existent _id");

    Ok(())
}
