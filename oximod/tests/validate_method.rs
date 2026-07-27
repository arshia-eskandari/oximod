mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod validators {
    pub fn validate_name(value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("name cannot be blank".into());
        }

        if value.eq_ignore_ascii_case("admin") {
            return Err("name 'admin' is reserved".into());
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Role {
    Admin,
    User,
    Guess,
}

// Run test: cargo nextest run test_validate_returns_unit_on_success_and_err_on_failure
#[tokio::test]
async fn test_validate_returns_unit_on_success_and_err_on_failure() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_returns_unit_on_success_and_err_on_failure")]
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

    let valid_user = User::default()
        .name("User123456")
        .email("x@y.com")
        .role(Role::Admin);

    let result = valid_user.validate();
    assert!(matches!(result, Ok(())));

    let invalid_user = User::default()
        .name("abc")
        .email("x@y.com")
        .role(Role::Admin);

    let err = invalid_user.validate();
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 5"));

    Ok(())
}

// Run test: cargo nextest run test_multiple_validation_errors
#[tokio::test]
async fn test_multiple_validation_errors() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("multiple_validation_errors")]
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

    let invalid_user = User::default().name("abc").email("not-an-email");

    let err = invalid_user.validate();
    assert!(err.is_err());

    let err_str = format!("{:?}", err);

    assert!(err_str.contains("at least 5"));
    assert!(err_str.contains("email"));
    assert!(err_str.contains("required"));

    Ok(())
}

// Run test: cargo nextest run test_validate_custom_validator_returns_unit_on_success_and_err_on_failure
#[tokio::test]
async fn test_validate_custom_validator_returns_unit_on_success_and_err_on_failure() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_validator_returns_unit_on_success_and_err_on_failure")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_name))]
        name: String,
    }

    User::clear().await?;

    let valid_user = User::default().name("User1");
    let result = valid_user.validate();
    assert!(matches!(result, Ok(())));

    let invalid_user = User::default().name("admin");
    let err = invalid_user.validate();

    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("name 'admin' is reserved"));

    Ok(())
}
