mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{ Deserialize, Serialize };
use testresult::TestResult;

// Run test: cargo nextest run test_multiple_of_passes
#[tokio::test]
async fn test_multiple_of_passes() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_multiple_of_passes")]
    struct Product {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(multiple_of = 5)]
        quantity: u64,
    }

    Product::clear().await?;

    let product = Product::default().quantity(10); // ✅ divisible by 5
    let result = product.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_multiple_of_fails
#[tokio::test]
async fn test_multiple_of_fails() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_multiple_of_fails")]
    struct Product {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(multiple_of = 5)]
        quantity: i64,
    }

    Product::clear().await?;

    let product = Product::default().quantity(7); // ❌ not divisible by 5
    let result = product.save().await;

    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("must be a multiple of"));
    Ok(())
}

// Run test: cargo nextest run test_multiple_of_passes_non_optional
#[tokio::test]
async fn test_multiple_of_passes_non_optional() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_multiple_of_passes_non_optional")]
    struct Product {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(multiple_of = 5)]
        quantity: u64,
    }

    Product::clear().await?;

    let product = Product::default().quantity(10); // ✅ divisible by 5
    let result = product.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_multiple_of_fails_non_optional
#[tokio::test]
async fn test_multiple_of_fails_non_optional() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_multiple_of_fails_non_optional")]
    struct Product {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(multiple_of = 5)]
        quantity: i64,
    }

    Product::clear().await?;

    let product = Product::default().quantity(7); // ❌ not divisible by 5
    let result = product.save().await;

    assert!(result.is_err());
    assert!(format!("{:?}", result).contains("must be a multiple of"));
    Ok(())
}
