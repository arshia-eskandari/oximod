mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

// Run test: cargo nextest run test_positive_passes
#[tokio::test]
async fn test_positive_passes() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_positive_pass")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(positive)]
        value: i64,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(42);
    let result = doc.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_positive_fails
#[tokio::test]
async fn test_positive_fails() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_positive_fail")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(positive)]
        value: i64,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(0);
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("positive"));
    Ok(())
}

// Run test: cargo nextest run test_negative_passes
#[tokio::test]
async fn test_negative_passes() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_negative_pass")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(negative)]
        value: i64,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(-42);
    let result = doc.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_negative_fails
#[tokio::test]
async fn test_negative_fails() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_negative_fail")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(negative)]
        value: i64,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(1);
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("negative"));
    Ok(())
}

// Run test: cargo nextest run test_non_negative_passes
#[tokio::test]
async fn test_non_negative_passes() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_non_negative_pass")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(non_negative)]
        value: i64,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(0);
    let result = doc.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_non_negative_fails
#[tokio::test]
async fn test_non_negative_fails() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_non_negative_fail")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(non_negative)]
        value: i64,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(-1);
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("non-negative"));
    Ok(())
}

// Run test: cargo nextest run test_positive_passes_non_optional
#[tokio::test]
async fn test_positive_passes_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_positive_pass_non_optional")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(positive)]
        value: i32,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(42);
    let result = doc.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_positive_fails_non_optional
#[tokio::test]
async fn test_positive_fails_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_positive_fail_non_optional")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(positive)]
        value: i16,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(0i16);
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("positive"));
    Ok(())
}

// Run test: cargo nextest run test_negative_passes_non_optional
#[tokio::test]
async fn test_negative_passes_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_negative_pass_non_optional")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(negative)]
        value: i32,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(-42);
    let result = doc.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_negative_fails_non_optional
#[tokio::test]
async fn test_negative_fails_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_negative_fail_non_optional")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(negative)]
        value: i64,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(1);
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("negative"));
    Ok(())
}

// Run test: cargo nextest run test_non_negative_passes_non_optional
#[tokio::test]
async fn test_non_negative_passes_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_non_negative_pass_non_optional")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(non_negative)]
        value: isize,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(0isize);
    let result = doc.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_non_negative_fails_non_optional
#[tokio::test]
async fn test_non_negative_fails_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_non_negative_fail_non_optional")]
    struct NumericValidation {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(non_negative)]
        value: isize,
    }

    NumericValidation::clear().await?;

    let doc = NumericValidation::default().value(-1isize);
    let result = doc.save().await;
    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("non-negative"));
    Ok(())
}
