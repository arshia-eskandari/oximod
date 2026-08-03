//! Integration tests for regular-expression pattern validation.
//!
//! The tests cover matching and nonmatching values for optional and
//! nonoptional string fields.

mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

// Run test: cargo nextest run test_invalid_pattern_format
#[tokio::test]
async fn test_invalid_pattern_format() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_pattern_invalid")]
    struct Product {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(pattern = r"^SKU-\d{4}$")]
        code: Option<String>,

        #[validate(non_empty)]
        name: Option<String>,

        #[validate(positive)]
        quantity: i32,

        #[validate(negative)]
        temperature: i32,

        #[validate(non_negative)]
        rating: i32,
    }

    Product::clear().await?;

    let product = Product::default()
        .code("BAD-SKU") // ❌ does not match ^SKU-\d{4}$
        .name("Product1")
        .quantity(10)
        .temperature(-10)
        .rating(5);

    let err = product.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("pattern"));
    Ok(())
}

// Run test: cargo nextest run test_valid_pattern_format
#[tokio::test]
async fn test_valid_pattern_format() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_pattern_valid")]
    struct Product {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(pattern = r"^SKU-\d{4}$")]
        code: Option<String>,

        #[validate(non_empty)]
        name: Option<String>,

        #[validate(positive)]
        quantity: i32,

        #[validate(negative)]
        temperature: i32,

        #[validate(non_negative)]
        rating: i32,
    }

    Product::clear().await?;

    let product = Product::default()
        .code("SKU-1234") // ✅ matches pattern
        .name("Product1")
        .quantity(10)
        .temperature(-10)
        .rating(5);

    let result = product.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_invalid_pattern_format_non_optional
#[tokio::test]
async fn test_invalid_pattern_format_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_pattern_invalid_non_optional")]
    struct Product {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(pattern = r"^SKU-\d{4}$")]
        code: String,

        #[validate(non_empty)]
        name: Option<String>,

        #[validate(positive)]
        quantity: i32,

        #[validate(negative)]
        temperature: i32,

        #[validate(non_negative)]
        rating: i32,
    }

    Product::clear().await?;

    let product = Product::default()
        .code("BAD-SKU") // ❌ does not match ^SKU-\d{4}$
        .name("Product1")
        .quantity(10)
        .temperature(-10)
        .rating(5);

    let err = product.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("pattern"));
    Ok(())
}

// Run test: cargo nextest run test_valid_pattern_format_non_optional
#[tokio::test]
async fn test_valid_pattern_format_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_pattern_valid_non_optional")]
    struct Product {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(pattern = r"^SKU-\d{4}$")]
        code: String,

        #[validate(non_empty)]
        name: Option<String>,

        #[validate(positive)]
        quantity: i32,

        #[validate(negative)]
        temperature: i32,

        #[validate(non_negative)]
        rating: i32,
    }

    Product::clear().await?;

    let product = Product::default()
        .code("SKU-1234") // ✅ matches pattern
        .name("Product1")
        .quantity(10)
        .temperature(-10)
        .rating(5);

    let result = product.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}
