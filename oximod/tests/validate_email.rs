//! Integration tests for email-address validation.
//!
//! The tests cover valid and malformed addresses for optional and nonoptional
//! string fields, including missing `@` signs and incomplete domains.

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

// Run test: cargo nextest run test_missing_at_symbol
#[tokio::test]
async fn test_missing_at_symbol() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_email_missing_at")]
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

    User::clear().await?;

    let user = User::default()
        .name("Valid")
        .email("invalidemail.com")
        .role(Role::Admin);

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("valid email"));
    Ok(())
}

// Run test: cargo nextest run test_missing_domain_dot
#[tokio::test]
async fn test_missing_domain_dot() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_email_missing_dot")]
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

    User::clear().await?;

    let user = User::default()
        .name("Valid")
        .email("user@domain")
        .role(Role::Admin);

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("valid email"));
    Ok(())
}

// Run test: cargo nextest run test_valid_email
#[tokio::test]
async fn test_valid_email() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_email_valid")]
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

    User::clear().await?;

    let user = User::default()
        .name("Valid")
        .email("user@example.com")
        .role(Role::Guess);

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_missing_at_symbol_non_optional
#[tokio::test]
async fn test_missing_at_symbol_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_email_missing_at_non_optional")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, max_length = 10)]
        name: String,

        #[validate(email)]
        email: String,

        #[validate(required)]
        role: Option<Role>,
    }

    User::clear().await?;

    let user = User::default()
        .name("Valid")
        .email("invalidemail.com")
        .role(Role::Admin);

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("valid email"));
    Ok(())
}

// Run test: cargo nextest run test_missing_domain_dot_non_optional
#[tokio::test]
async fn test_missing_domain_dot_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_email_missing_dot_non_optional")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, max_length = 10)]
        name: String,

        #[validate(email)]
        email: String,

        #[validate(required)]
        role: Option<Role>,
    }

    User::clear().await?;

    let user = User::default()
        .name("Valid")
        .email("user@domain")
        .role(Role::Admin);

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("valid email"));
    Ok(())
}

// Run test: cargo nextest run test_valid_email_non_optional
#[tokio::test]
async fn test_valid_email_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_email_valid_non_optional")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, max_length = 10)]
        name: String,

        #[validate(email)]
        email: String,

        #[validate(required)]
        role: Option<Role>,
    }

    User::clear().await?;

    let user = User::default()
        .name("Valid")
        .email("user@example.com")
        .role(Role::Guess);

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}
