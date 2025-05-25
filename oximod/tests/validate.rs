use mongodb::bson::oid::ObjectId;
use oximod::{set_global_client, Model};
use testresult::TestResult;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("validation_test")]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(min_length = 5, max_length = 10)]
    name: String,

    #[validate(email)]
    email: Option<String>,

    #[validate(required, enum_values("admin", "user", "guest"))]
    role: Option<String>,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("pattern_test")]
pub struct Product {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(pattern = r"^SKU-\d{4}$")]
    code: Option<String>,

    #[validate(non_empty)]
    name: Option<String>,
}

async fn init() {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
    set_global_client(mongodb_uri).await.unwrap();
}

// =======================
//      User tests
// =======================

// Run test: cargo nextest run test_valid_user_saves_successfully
#[tokio::test]
async fn test_valid_user_saves_successfully() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "ValidName".to_string(),
        email: Some("user@example.com".to_string()),
        role: Some("user".to_string()),
    };

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());
    Ok(())
}

// Run test: cargo nextest run test_min_length_violation
#[tokio::test]
async fn test_min_length_violation() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "abc".to_string(),
        email: Some("x@y.com".to_string()),
        role: Some("user".to_string()),
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
        name: "ThisNameIsWayTooLong".to_string(),
        email: Some("x@y.com".to_string()),
        role: Some("user".to_string()),
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most"));
    Ok(())
}

// Run test: cargo nextest run test_missing_required_field
#[tokio::test]
async fn test_missing_required_field() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "Valid".to_string(),
        email: None,
        role: None,
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("is required"));
    Ok(())
}

// Run test: cargo nextest run test_invalid_enum_value
#[tokio::test]
async fn test_invalid_enum_value() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("user@example.com".to_string()),
        role: Some("superuser".to_string()),
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must be one of"));
    Ok(())
}

// Run test: cargo nextest run test_missing_required_role
#[tokio::test]
async fn test_missing_required_role() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("user@example.com".to_string()),
        role: None,
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("is required"));
    Ok(())
}

// Run test: cargo nextest run test_invalid_email_missing_at
#[tokio::test]
async fn test_invalid_email_missing_at() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("invalidemail.com".to_string()),
        role: Some("admin".to_string()),
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("valid email"));
    Ok(())
}

// Run test: cargo nextest run test_invalid_email_missing_domain_dot
#[tokio::test]
async fn test_invalid_email_missing_domain_dot() -> TestResult {
    init().await;
    User::clear().await?;

    let user = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("user@domain".to_string()),
        role: Some("admin".to_string()),
    };

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("valid email"));
    Ok(())
}

// =======================
//      Product test
// =======================

// Run test: cargo nextest run test_invalid_pattern_format
#[tokio::test]
async fn test_invalid_pattern_format() -> TestResult {
    init().await;
    Product::clear().await?;

    let product = Product {
        _id: None,
        code: Some("BAD-SKU".to_string()),
        name: Some("Product1".to_string()),
    };

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

    let product = Product {
        _id: None,
        code: Some("SKU-1234".to_string()), // ✅ matches ^SKU-\d{4}$
        name: Some("Product1".to_string()),
    };

    let result = product.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_non_empty_field_missing_or_blank
#[tokio::test]
async fn test_non_empty_field_missing_or_blank() -> TestResult {
    init().await;
    Product::clear().await?;

    // Case 1: Field is None
    let missing_name = Product {
        _id: None,
        code: Some("SKU-1234".to_string()),
        name: None,
    };

    let err = missing_name.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("non-empty"));

    // Case 2: Field is Some("") (empty string)
    let empty_name = Product {
        _id: None,
        code: Some("SKU-1234".to_string()),
        name: Some("".to_string()),
    };

    let err = empty_name.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("non-empty"));

    // Case 3: Field is Some("   ") (whitespace only)
    let whitespace_name = Product {
        _id: None,
        code: Some("SKU-1234".to_string()),
        name: Some("   ".to_string()),
    };

    let err = whitespace_name.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("non-empty"));

    Ok(())
}

// Run test: cargo nextest run test_non_empty_field_valid
#[tokio::test]
async fn test_non_empty_field_valid() -> TestResult {
    init().await;
    Product::clear().await?;

    let valid = Product {
        _id: None,
        code: Some("SKU-1234".to_string()),
        name: Some("Non-Empty Name".to_string()),
    };

    let result = valid.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}
