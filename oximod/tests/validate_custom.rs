mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use testresult::TestResult;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Admin,
    User,
    Guest,
}

mod validators {
    use super::{Cow, HashMap, HashSet, Role};

    pub fn validate_name(value: &String) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("name cannot be blank".into());
        }

        if value.eq_ignore_ascii_case("admin") {
            return Err("name 'admin' is reserved".into());
        }

        Ok(())
    }

    pub fn validate_age(value: &i32) -> Result<(), String> {
        if *value < 18 {
            return Err("age must be at least 18".into());
        }

        if *value > 120 {
            return Err("age must be realistic".into());
        }

        Ok(())
    }

    pub fn validate_nickname(value: &Cow<'static, str>) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("nickname cannot be blank".into());
        }

        if value.contains(' ') {
            return Err("nickname cannot contain spaces".into());
        }

        Ok(())
    }

    pub fn validate_tags(value: &Vec<String>) -> Result<(), String> {
        if value.is_empty() {
            return Err("tags cannot be empty".into());
        }

        if value.iter().any(|tag| tag.trim().is_empty()) {
            return Err("tags cannot contain blank values".into());
        }

        Ok(())
    }

    pub fn validate_unique_tags(value: &HashSet<String>) -> Result<(), String> {
        if value.contains("forbidden") {
            return Err("tag 'forbidden' is not allowed".into());
        }

        Ok(())
    }

    pub fn validate_role(value: &Role) -> Result<(), String> {
        match value {
            Role::Guest => Err("guest role is not allowed for persisted users".into()),
            _ => Ok(()),
        }
    }

    pub fn validate_nested_scores(value: &Vec<Vec<i32>>) -> Result<(), String> {
        if value.is_empty() {
            return Err("scores cannot be empty".into());
        }

        if value.iter().any(|inner| inner.is_empty()) {
            return Err("scores cannot contain empty inner vectors".into());
        }

        if value.iter().flatten().any(|score| *score < 0) {
            return Err("scores cannot contain negative values".into());
        }

        Ok(())
    }

    pub fn validate_profile_metadata(value: &HashMap<String, Vec<String>>) -> Result<(), String> {
        if value.is_empty() {
            return Err("metadata cannot be empty".into());
        }

        if value.keys().any(|k| k.trim().is_empty()) {
            return Err("metadata keys cannot be blank".into());
        }

        if value.values().any(|v| v.is_empty()) {
            return Err("metadata values cannot contain empty lists".into());
        }

        if value.values().flatten().any(|s| s.trim().is_empty()) {
            return Err("metadata values cannot contain blank strings".into());
        }

        Ok(())
    }
}

// Run test: cargo nextest run test_custom_string_validation_violation
#[tokio::test]
async fn test_custom_string_validation_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_string_violation")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_name))]
        name: String,
    }

    User::clear().await?;

    let reserved = User::default().name("admin");

    let err = reserved.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("name 'admin' is reserved"));

    Ok(())
}

// Run test: cargo nextest run test_custom_string_validation_valid
#[tokio::test]
async fn test_custom_string_validation_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_string_valid")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_name))]
        name: String,
    }

    User::clear().await?;

    let user = User::default().name("Arshia");

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_custom_numeric_validation_violation
#[tokio::test]
async fn test_custom_numeric_validation_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_numeric_violation")]
    struct Person {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_age))]
        age: i32,
    }

    Person::clear().await?;

    let person = Person::default().age(16);

    let err = person.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("age must be at least 18"));

    Ok(())
}

// Run test: cargo nextest run test_custom_numeric_validation_valid
#[tokio::test]
async fn test_custom_numeric_validation_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_numeric_valid")]
    struct Person {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_age))]
        age: i32,
    }

    Person::clear().await?;

    let person = Person::default().age(30);

    let result = person.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_custom_and_builtin_string_builtin_violation
#[tokio::test]
async fn test_custom_and_builtin_string_builtin_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_and_builtin_string_builtin_violation")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, custom(crate::validators::validate_name))]
        name: String,
    }

    User::clear().await?;

    let user = User::default().name("abc");

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 5"));

    Ok(())
}

// Run test: cargo nextest run test_custom_and_builtin_string_custom_violation
#[tokio::test]
async fn test_custom_and_builtin_string_custom_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_and_builtin_string_custom_violation")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, custom(crate::validators::validate_name))]
        name: String,
    }

    User::clear().await?;

    let user = User::default().name("admin");

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("name 'admin' is reserved"));

    Ok(())
}

// Run test: cargo nextest run test_custom_and_builtin_string_valid
#[tokio::test]
async fn test_custom_and_builtin_string_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_and_builtin_string_valid")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, custom(crate::validators::validate_name))]
        name: String,
    }

    User::clear().await?;

    let user = User::default().name("ValidUser");

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_custom_optional_string_some_violation
#[tokio::test]
async fn test_custom_optional_string_some_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_optional_string_some_violation")]
    struct Profile {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_name))]
        display_name: Option<String>,
    }

    Profile::clear().await?;

    let profile = Profile::default().display_name("admin");

    let err = profile.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("name 'admin' is reserved"));

    Ok(())
}

// Run test: cargo nextest run test_custom_optional_string_none_skips_validation
#[tokio::test]
async fn test_custom_optional_string_none_skips_validation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_optional_string_none_skips_validation")]
    struct Profile {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_name))]
        display_name: Option<String>,
    }

    Profile::clear().await?;

    let profile = Profile::default();

    let result = profile.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_custom_optional_string_required_none_violation
#[tokio::test]
async fn test_custom_optional_string_required_none_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_optional_string_required_none_violation")]
    struct Profile {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(required, custom(crate::validators::validate_name))]
        display_name: Option<String>,
    }

    Profile::clear().await?;

    let profile = Profile::default();

    let err = profile.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("required"));

    Ok(())
}

// Run test: cargo nextest run test_custom_cow_validation_violation
#[tokio::test]
async fn test_custom_cow_validation_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_cow_violation")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_nickname))]
        nickname: Cow<'static, str>,
    }

    User::clear().await?;

    let user = User::default().nickname("bad nick");

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("nickname cannot contain spaces"));

    Ok(())
}

// Run test: cargo nextest run test_custom_cow_validation_valid
#[tokio::test]
async fn test_custom_cow_validation_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_cow_valid")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_nickname))]
        nickname: Cow<'static, str>,
    }

    User::clear().await?;

    let user = User::default().nickname("ValidNick");

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_custom_vec_validation_violation
#[tokio::test]
async fn test_custom_vec_validation_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_vec_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_tags))]
        tags: Vec<String>,
    }

    Book::clear().await?;

    let book = Book::default().tags(vec!["rust".to_string(), "".to_string()]);

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("tags cannot contain blank values"));

    Ok(())
}

// Run test: cargo nextest run test_custom_vec_validation_with_builtin_length_violation
#[tokio::test]
async fn test_custom_vec_validation_with_builtin_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_vec_with_builtin_length_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(
            min_length = 2,
            max_length = 4,
            custom(crate::validators::validate_tags)
        )]
        tags: Vec<String>,
    }

    Book::clear().await?;

    let book = Book::default().tags(vec!["rust".to_string()]);

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 2"));

    Ok(())
}

// Run test: cargo nextest run test_custom_vec_validation_valid
#[tokio::test]
async fn test_custom_vec_validation_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_vec_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(
            min_length = 2,
            max_length = 4,
            custom(crate::validators::validate_tags)
        )]
        tags: Vec<String>,
    }

    Book::clear().await?;

    let book = Book::default().tags(vec!["rust".to_string(), "mongodb".to_string()]);

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_custom_hashset_validation_violation
#[tokio::test]
async fn test_custom_hashset_validation_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_hashset_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_unique_tags))]
        tags: HashSet<String>,
    }

    Book::clear().await?;

    let book = Book::default().tags(HashSet::from(["rust".to_string(), "forbidden".to_string()]));

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("tag 'forbidden' is not allowed"));

    Ok(())
}

// Run test: cargo nextest run test_custom_hashset_validation_valid
#[tokio::test]
async fn test_custom_hashset_validation_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_hashset_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_unique_tags))]
        tags: HashSet<String>,
    }

    Book::clear().await?;

    let book = Book::default().tags(HashSet::from(["rust".to_string(), "mongodb".to_string()]));

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_custom_enum_optional_required_and_custom_violation
#[tokio::test]
async fn test_custom_enum_optional_required_and_custom_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_enum_optional_required_and_custom_violation")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(required, custom(crate::validators::validate_role))]
        role: Option<Role>,
    }

    User::clear().await?;

    let user = User::default().role(Role::Guest);

    let err = user.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("guest role is not allowed"));

    Ok(())
}

// Run test: cargo nextest run test_custom_enum_optional_required_and_custom_valid
#[tokio::test]
async fn test_custom_enum_optional_required_and_custom_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_enum_optional_required_and_custom_valid")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(required, custom(crate::validators::validate_role))]
        role: Option<Role>,
    }

    User::clear().await?;

    let user = User::default().role(Role::Admin);

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_custom_nested_vec_violation
#[tokio::test]
async fn test_custom_nested_vec_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_nested_vec_violation")]
    struct Report {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_nested_scores))]
        scores: Vec<Vec<i32>>,
    }

    Report::clear().await?;

    let report = Report::default().scores(vec![vec![1, 2], vec![]]);

    let err = report.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("scores cannot contain empty inner vectors"));

    Ok(())
}

// Run test: cargo nextest run test_custom_nested_vec_with_builtin_length_violation
#[tokio::test]
async fn test_custom_nested_vec_with_builtin_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_nested_vec_with_builtin_length_violation")]
    struct Report {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, custom(crate::validators::validate_nested_scores))]
        scores: Vec<Vec<i32>>,
    }

    Report::clear().await?;

    let report = Report::default().scores(vec![vec![1, 2]]);

    let err = report.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 2"));

    Ok(())
}

// Run test: cargo nextest run test_custom_nested_vec_valid
#[tokio::test]
async fn test_custom_nested_vec_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_nested_vec_valid")]
    struct Report {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, custom(crate::validators::validate_nested_scores))]
        scores: Vec<Vec<i32>>,
    }

    Report::clear().await?;

    let report = Report::default().scores(vec![vec![1, 2], vec![3, 4]]);

    let result = report.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_custom_nested_hashmap_violation
#[tokio::test]
async fn test_custom_nested_hashmap_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_nested_hashmap_violation")]
    struct Profile {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_profile_metadata))]
        metadata: HashMap<String, Vec<String>>,
    }

    Profile::clear().await?;

    let profile = Profile::default().metadata(HashMap::from([(
        "skills".to_string(),
        vec!["rust".to_string(), "".to_string()],
    )]));

    let err = profile.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("metadata values cannot contain blank strings"));

    Ok(())
}

// Run test: cargo nextest run test_custom_nested_hashmap_valid
#[tokio::test]
async fn test_custom_nested_hashmap_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_custom_nested_hashmap_valid")]
    struct Profile {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(custom(crate::validators::validate_profile_metadata))]
        metadata: HashMap<String, Vec<String>>,
    }

    Profile::clear().await?;

    let profile = Profile::default().metadata(HashMap::from([
        (
            "skills".to_string(),
            vec!["rust".to_string(), "mongodb".to_string()],
        ),
        ("tools".to_string(), vec!["cargo".to_string()]),
    ]));

    let result = profile.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_multiple_custom_validators_same_model
#[tokio::test]
async fn test_multiple_custom_validators_same_model() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_multiple_custom_validators_same_model")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, custom(crate::validators::validate_name))]
        name: String,

        #[validate(custom(crate::validators::validate_age))]
        age: i32,

        #[validate(required, custom(crate::validators::validate_role))]
        role: Option<Role>,
    }

    User::clear().await?;

    let user = User::default().name("ValidUser").age(28).role(Role::Admin);

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_multiple_custom_validators_same_model_first_failing_field
#[tokio::test]
async fn test_multiple_custom_validators_same_model_first_failing_field() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_multiple_custom_validators_same_model_first_failing_field")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, custom(crate::validators::validate_name))]
        name: String,

        #[validate(custom(crate::validators::validate_age))]
        age: i32,

        #[validate(required, custom(crate::validators::validate_role))]
        role: Option<Role>,
    }

    User::clear().await?;

    let user = User::default().name("admin").age(16).role(Role::Guest);

    let err = user.save().await;
    assert!(err.is_err());

    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("name 'admin' is reserved")
            || err_str.contains("age must be at least 18")
            || err_str.contains("guest role is not allowed")
    );

    Ok(())
}
