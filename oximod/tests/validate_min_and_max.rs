mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};
use testresult::TestResult;

// Run test: cargo nextest run test_book_min_violation
#[tokio::test]
async fn test_book_min_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_min_only")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min = 100)]
        pages: i32,

        #[validate(max = 100)]
        price: i32,

        #[validate(min = 1900, max = 2025)]
        year: i32,
    }

    Book::clear().await?;

    let book = Book::default()
        .pages(99) // ❌ below min
        .price(80)
        .year(2000);

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at least 100"));
    Ok(())
}

// Run test: cargo nextest run test_book_min_exclusive_violation
#[tokio::test]
async fn test_book_min_exclusive_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_min_exclusive_only")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min = 100, min_exclusive)]
        pages: i32,

        #[validate(max = 100)]
        price: i32,

        #[validate(min = 1900, max = 2025)]
        year: i32,
    }

    Book::clear().await?;

    let book = Book::default()
        .pages(100) // ❌ cannot be equal to min
        .price(80)
        .year(2000);

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("more than 100"));
    Ok(())
}

// Run test: cargo nextest run test_book_max_violation
#[tokio::test]
async fn test_book_max_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_max_only")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min = 100)]
        pages: i32,

        #[validate(max = 100)]
        price: i32,

        #[validate(min = 1900, max = 2025)]
        year: i32,
    }

    Book::clear().await?;

    let book = Book::default()
        .pages(120)
        .price(150) // ❌ above max
        .year(2000);

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 100"));
    Ok(())
}

// Run test: cargo nextest run test_book_max_exclusive_violation
#[tokio::test]
async fn test_book_max_exclusive_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_max_exclusive_only")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min = 100)]
        pages: i32,

        #[validate(max = 100, max_exclusive)]
        price: i32,

        #[validate(min = 1900, max = 2025)]
        year: i32,
    }

    Book::clear().await?;

    let book = Book::default()
        .pages(120)
        .price(100) // ❌ cannot be equal to max
        .year(2000);

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("less than 100"));
    Ok(())
}

// Run test: cargo nextest run test_book_valid
#[tokio::test]
async fn test_book_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_book_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min = 100)]
        pages: i32,

        #[validate(max = 100)]
        price: i32,

        #[validate(min = 1900, max = 2025)]
        year: i32,
    }

    Book::clear().await?;

    let book = Book::default().pages(120).price(90).year(2022);

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_book_min_violation_non_optional
#[tokio::test]
async fn test_book_min_violation_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_min_only_non_optional")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min = 100)]
        pages: i32,

        #[validate(max = 100)]
        price: i32,

        #[validate(min = 1900, max = 2025)]
        year: i32,
    }

    Book::clear().await?;

    let book = Book::default()
        .pages(99) // ❌ below min
        .price(80)
        .year(2000);

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at least 100"));
    Ok(())
}

// Run test: cargo nextest run test_book_max_violation_non_optional
#[tokio::test]
async fn test_book_max_violation_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_max_only_non_optional")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min = 100)]
        pages: i32,

        #[validate(max = 100)]
        price: i32,

        #[validate(min = 1900, max = 2025)]
        year: i32,
    }

    Book::clear().await?;

    let book = Book::default()
        .pages(120)
        .price(150) // ❌ above max
        .year(2000);

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 100"));
    Ok(())
}

// Run test: cargo nextest run test_book_valid_non_optional
#[tokio::test]
async fn test_book_valid_non_optional() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_book_valid_non_optional")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min = 100)]
        pages: i32,

        #[validate(max = 100)]
        price: i32,

        #[validate(min = 1900, max = 2025)]
        year: i32,
    }

    Book::clear().await?;

    let book = Book::default().pages(120).price(90).year(2022);

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}
