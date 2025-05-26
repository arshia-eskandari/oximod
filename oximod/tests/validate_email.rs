mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

#[derive(Serialize, Deserialize, Debug)]
pub enum Role {
    Admin,
    User,
    Guess,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("validate_email")]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(min_length = 5, max_length = 10)]
    name: String,

    #[validate(email)]
    email: Option<String>,

    #[validate(required)]
    role: Option<Role>,
}

// Run test: cargo nextest run test_missing_at_symbol
#[tokio::test]
async fn test_missing_at_symbol() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("invalidemail.com".to_string()), // ❌ missing '@'
        role: Some(Role::Admin),
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("valid email"));
    Ok(())
}

// Run test: cargo nextest run test_missing_domain_dot
#[tokio::test]
async fn test_missing_domain_dot() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("user@domain".to_string()), // ❌ missing .com, .net, etc.
        role: Some(Role::Admin),
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("valid email"));
    Ok(())
}

// Run test: cargo nextest run test_valid_email
#[tokio::test]
async fn test_valid_email() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("user@example.com".to_string()), // ✅ valid
        role: Some(Role::Guess),
    };

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}
