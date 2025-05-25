mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("validate_min_and_max")]
pub struct Book {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(min = 100)]
    pages: i32, 

    #[validate(max = 100)]
    price: i32, 

    #[validate(min = 1900, max = 2025)]
    year: i32, 
}

// Run test: cargo nextest run test_book_min_violation
#[tokio::test]
async fn test_book_min_violation() -> TestResult {
    init().await;
    Book::clear().await?;

    let book = Book {
        _id: None,
        pages: 99, // ❌ below min
        price: 80,
        year: 2000,
    };

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at least 100"));
    Ok(())
}

// Run test: cargo nextest run test_book_max_violation
#[tokio::test]
async fn test_book_max_violation() -> TestResult {
    init().await;
    Book::clear().await?;

    let book = Book {
        _id: None,
        pages: 120,
        price: 150, // ❌ above max
        year: 2000,
    };

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 100"));
    Ok(())
}

// Run test: cargo nextest run test_book_valid
#[tokio::test]
async fn test_book_valid() -> TestResult {
    init().await;
    Book::clear().await?;

    let book = Book {
        _id: None,
        pages: 120,
        price: 90,
        year: 2022,
    };

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}