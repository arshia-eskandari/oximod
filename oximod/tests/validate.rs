use mongodb::bson::oid::ObjectId;
use oximod::{set_global_client, Model};
use testresult::TestResult;
use serde::{Deserialize, Serialize};

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
    }

    User::clear().await?;

    let valid_user = User {
        _id: None,
        name: "ValidName".to_string(),
    };

    let result = valid_user.save().await?;
    assert_ne!(result, ObjectId::default());

    let too_short_user = User {
        _id: None,
        name: "abc".to_string(), // Too short
    };

    let err = too_short_user.save().await;
    assert!(err.is_err());
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("must be at least 5 characters long"),
        "Expected validation error message, got: {msg}"
    );

    let too_long_user = User {
      _id: None,
      name: "ThisNameIsWayTooLong".to_string(),
    };
    
    let err = too_long_user.save().await;
    assert!(err.is_err());
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("at most"),
        "Expected max length validation error, got: {msg}"
    ); 

    Ok(())
}
