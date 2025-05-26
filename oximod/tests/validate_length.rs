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
#[collection("validate_length")]
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

// Run test: cargo nextest run test_min_length_violation
#[tokio::test]
async fn test_min_length_violation() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "abc".to_string(), // too short
        email: Some("x@y.com".to_string()),
        role: Some(Role::Admin),
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at least 5 characters"));

    Ok(())
}

// Run test: cargo nextest run test_max_length_violation
#[tokio::test]
async fn test_max_length_violation() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "ThisNameIsWayTooLong".to_string(), // too long
        email: Some("x@y.com".to_string()),
        role: Some(Role::Admin),
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most"));

    Ok(())
}

// Run test: cargo nextest run test_length_valid
#[tokio::test]
async fn test_length_valid() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "ValidName".to_string(), // valid
        email: Some("user@example.com".to_string()),
        role: Some(Role::Admin),
    };

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}
