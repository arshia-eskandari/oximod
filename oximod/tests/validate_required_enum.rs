mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{ Deserialize, Serialize };
use testresult::TestResult;

#[derive(Serialize, Deserialize, Debug)]
pub enum Role {
    Admin,
    User,
    Guess,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("validate_required_enum")]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(min_length = 5, max_length = 10)]
    name: String,

    #[validate(required, email)]
    email: Option<String>,

    #[validate(required)]
    role: Option<Role>,
}

// Run test: cargo nextest run test_missing_required_email
#[tokio::test]
async fn test_missing_required_email() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User::default().name("Valid".to_string()).role(Role::User); // ❌ missing email

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("is required"));
    Ok(())
}

// Run test: cargo nextest run test_missing_required_role
#[tokio::test]
async fn test_missing_required_role() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User::default().name("Valid".to_string()).email("user@example.com".to_string()); // ❌ missing role

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("is required"));
    Ok(())
}

// Run test: cargo nextest run test_valid_required_enum
#[tokio::test]
async fn test_valid_required_enum() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User::default()
        .name("Valid".to_string())
        .email("user@example.com".to_string())
        .role(Role::Admin); // ✅ allowed enum

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}
