mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{ Deserialize, Serialize };
use testresult::TestResult;

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("validate_pattern")]
pub struct Product {
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

// Run test: cargo nextest run test_invalid_pattern_format
#[tokio::test]
async fn test_invalid_pattern_format() -> TestResult {
    init().await;
    Product::clear().await?;

    let product = Product::default()
        .code("BAD-SKU".to_string()) // ❌ does not match ^SKU-\d{4}$
        .name("Product1".to_string())
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
    init().await;
    Product::clear().await?;

    let product = Product::default()
        .code("SKU-1234".to_string()) // ✅ matches pattern
        .name("Product1".to_string())
        .quantity(10)
        .temperature(-10)
        .rating(5);

    let result = product.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}
