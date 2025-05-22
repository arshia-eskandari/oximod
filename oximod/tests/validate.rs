use mongodb::bson::oid::ObjectId;
use oximod::{ set_global_client, Model };
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

// Run test: cargo nextest run validates_correctly
#[tokio::test]
async fn validates_correctly() -> TestResult {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI").expect("Failed to find MONGODB_URI");

    set_global_client(mongodb_uri).await.unwrap_or_else(|e| panic!("{}", e));

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

    User::clear().await?;

    let valid_user = User {
        _id: None,
        name: "ValidName".to_string(),
        email: Some("user@example.com".to_string()),
        role: Some("user".to_string()),
    };

    let result = valid_user.save().await?;
    assert_ne!(result, ObjectId::default());

    let too_short_user = User {
        _id: None,
        name: "abc".to_string(),
        email: Some("x@y.com".to_string()),
        role: Some("user".to_string()),
    };

    let err = too_short_user.save().await;
    assert!(err.is_err());
    let msg = format!("{:?}", err);
    assert!(msg.contains("at least 5 characters"), "Expected min length error, got: {msg}");

    let too_long_user = User {
        _id: None,
        name: "ThisNameIsWayTooLong".to_string(),
        email: Some("x@y.com".to_string()),
        role: Some("user".to_string()),
    };

    let err = too_long_user.save().await;
    assert!(err.is_err());
    let msg = format!("{:?}", err);
    assert!(msg.contains("at most"), "Expected max length error, got: {msg}");

    let missing_required = User {
        _id: None,
        name: "Valid".to_string(),
        email: None,
        role: None,
    };

    let err = missing_required.save().await;
    assert!(err.is_err());
    let msg = format!("{:?}", err);
    assert!(msg.contains("is required"), "Expected required field error, got: {msg}");

    let invalid_enum = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("test@example.com".to_string()),
        role: Some("superuser".to_string()), // invalid
    };

    let err = invalid_enum.save().await;
    assert!(err.is_err());
    let msg = format!("{:?}", err);
    assert!(msg.contains("must be one of"), "Expected enum_values error, got: {msg}");

    let missing_role = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("valid@example.com".to_string()),
        role: None,
    };

    let err = missing_role.save().await;
    assert!(err.is_err());
    let msg = format!("{:?}", err);
    assert!(msg.contains("is required"), "Expected required field error for role, got: {msg}");

    let bad_email1 = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("notanemail.com".to_string()),
        role: Some("admin".to_string()),
    };

    let err = bad_email1.save().await;
    assert!(err.is_err());
    let msg = format!("{:?}", err);
    assert!(msg.contains("must be a valid email"), "Expected email validation error, got: {msg}");

    let bad_email2 = User {
        _id: None,
        name: "Valid".to_string(),
        email: Some("user@domain".to_string()),
        role: Some("admin".to_string()),
    };

    let err = bad_email2.save().await;
    assert!(err.is_err());
    let msg = format!("{:?}", err);
    assert!(msg.contains("must be a valid email"), "Expected email validation error, got: {msg}");

    Ok(())
}
