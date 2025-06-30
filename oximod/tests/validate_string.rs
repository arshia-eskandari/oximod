mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{ Deserialize, Serialize };
use testresult::TestResult;

// Run test: cargo nextest run test_valid_string_passes
#[tokio::test]
async fn test_valid_string_passes() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_string_valid")]
    struct StringValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(
            starts_with = "abc",
            ends_with = "xyz",
            includes = "middle",
            alphanumeric,
            non_empty
        )]
        value: Option<String>,
    }

    StringValidation::clear().await?;

    let doc = StringValidation::default().value("abcmiddlexyz".to_string());
    let result = doc.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_starts_with_fails
#[tokio::test]
async fn test_starts_with_fails() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_starts_with_fail")]
    struct StringValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(starts_with = "abc")]
        value: Option<String>,
    }

    StringValidation::clear().await?;

    let doc = StringValidation::default().value("wrongmiddlexyz".to_string());
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("must start with"));
    Ok(())
}

// Run test: cargo nextest run test_ends_with_fails
#[tokio::test]
async fn test_ends_with_fails() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_ends_with_fail")]
    struct StringValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(ends_with = "xyz")]
        value: Option<String>,
    }

    StringValidation::clear().await?;

    let doc = StringValidation::default().value("abcmiddlewrong".to_string());
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("must end with"));
    Ok(())
}

// Run test: cargo nextest run test_includes_fails
#[tokio::test]
async fn test_includes_fails() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_includes_fail")]
    struct StringValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(includes = "middle")]
        value: Option<String>,
    }

    StringValidation::clear().await?;

    let doc = StringValidation::default().value("abcnomatchxyz".to_string());
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("must include"));
    Ok(())
}

// Run test: cargo nextest run test_alphanumeric_fails
#[tokio::test]
async fn test_alphanumeric_fails() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_alphanumeric_fail")]
    struct StringValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(alphanumeric)]
        value: Option<String>,
    }

    StringValidation::clear().await?;

    let doc = StringValidation::default().value("abcmiddle!xyz".to_string());
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("must contain only alphanumeric"));
    Ok(())
}

// Run test: cargo nextest run test_empty_string_fails
#[tokio::test]
async fn test_empty_string_fails() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_non_empty_fail")]
    struct StringValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(non_empty)]
        value: Option<String>,
    }

    StringValidation::clear().await?;

    let doc = StringValidation::default().value("   ".to_string());
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("must be non-empty"));
    Ok(())
}
