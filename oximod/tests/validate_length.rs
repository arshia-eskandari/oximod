mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use testresult::TestResult;

#[derive(Serialize, Deserialize, Debug)]
pub enum Role {
    Admin,
    User,
    Guess,
}

// Run test: cargo nextest run test_string_length_violation
#[tokio::test]
async fn test_string_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_string_length_violation")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, max_length = 10)]
        name: String,

        #[validate(email)]
        email: Option<String>,

        #[validate(required)]
        role: Option<Role>,
    }

    User::clear().await?;

    let too_short = User::default()
        .name("abc") // ❌ below min
        .email("x@y.com")
        .role(Role::Admin);

    let err = too_short.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 5"));

    User::clear().await?;

    let too_long = User::default()
        .name("ThisNameIsWayTooLong") // ❌ above max
        .email("x@y.com")
        .role(Role::Admin);

    let err = too_long.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 10"));

    Ok(())
}

// Run test: cargo nextest run test_string_length_valid
#[tokio::test]
async fn test_string_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_string_length_valid")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, max_length = 10)]
        name: String,

        #[validate(email)]
        email: Option<String>,

        #[validate(required)]
        role: Option<Role>,
    }

    User::clear().await?;

    let user = User::default()
        .name("ValidName")
        .email("user@example.com")
        .role(Role::Admin);

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_cow_length_violation
#[tokio::test]
async fn test_cow_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_cow_length_violation")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, max_length = 10)]
        nickname: Cow<'static, str>,
    }

    User::clear().await?;

    let too_short = User::default().nickname("abc"); // ❌ below min

    let err = too_short.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 5"));

    User::clear().await?;

    let too_long = User::default().nickname("ThisNameIsWayTooLong"); // ❌ above max

    let err = too_long.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 10"));

    Ok(())
}

// Run test: cargo nextest run test_cow_length_valid
#[tokio::test]
async fn test_cow_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_cow_length_valid")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 5, max_length = 10)]
        nickname: Cow<'static, str>,
    }

    User::clear().await?;

    let user = User::default().nickname("ValidNick");

    let result = user.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_vec_length_violation
#[tokio::test]
async fn test_vec_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_vec_length_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 4)]
        tags: Vec<String>,
    }

    Book::clear().await?;

    let too_short = Book::default().tags(vec!["rust".to_string()]); // ❌ below min

    let err = too_short.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 2"));

    Book::clear().await?;

    let too_long = Book::default().tags(vec![
        "rust".to_string(),
        "mongodb".to_string(),
        "odm".to_string(),
        "macro".to_string(),
        "serde".to_string(), // ❌ above max
    ]);

    let err = too_long.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 4"));

    Ok(())
}

// Run test: cargo nextest run test_vec_length_valid
#[tokio::test]
async fn test_vec_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_vec_length_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 4)]
        tags: Vec<String>,
    }

    Book::clear().await?;

    let book = Book::default().tags(vec!["rust".to_string(), "mongodb".to_string()]);

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_optional_vec_length_violation
#[tokio::test]
async fn test_optional_vec_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_optional_vec_length_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        tags: Option<Vec<String>>,
    }

    Book::clear().await?;

    let too_short = Book::default().tags(vec!["rust".to_string()]); // ❌ below min

    let err = too_short.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 2"));

    Book::clear().await?;

    let too_long = Book::default().tags(vec![
        "rust".to_string(),
        "mongodb".to_string(),
        "odm".to_string(),
        "serde".to_string(), // ❌ above max
    ]);

    let err = too_long.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 3"));

    Ok(())
}

// Run test: cargo nextest run test_optional_vec_length_valid
#[tokio::test]
async fn test_optional_vec_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_optional_vec_length_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        tags: Option<Vec<String>>,
    }

    Book::clear().await?;

    let with_some = Book::default().tags(vec!["rust".to_string(), "mongodb".to_string()]);

    let result = with_some.save().await?;
    assert_ne!(result, ObjectId::default());

    Book::clear().await?;

    let with_none = Book::default();

    let result = with_none.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_array_length_violation
#[tokio::test]
async fn test_array_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_array_length_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 3, max_length = 3)]
        ratings: [i32; 3],
    }

    Book::clear().await?;

    let too_short = Book::default().ratings([4, 5, 6]); // compile-time fixed length, uses min/max-equal shape

    let result = too_short.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_array_min_violation
#[tokio::test]
async fn test_array_min_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_array_min_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 4)]
        ratings: [i32; 3],
    }

    Book::clear().await?;

    let book = Book::default().ratings([4, 5, 6]); // ❌ below min

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 4"));

    Ok(())
}

// Run test: cargo nextest run test_array_max_violation
#[tokio::test]
async fn test_array_max_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_array_max_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(max_length = 2)]
        ratings: [i32; 3],
    }

    Book::clear().await?;

    let book = Book::default().ratings([4, 5, 6]); // ❌ above max

    let err = book.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 2"));

    Ok(())
}

// Run test: cargo nextest run test_array_length_valid
#[tokio::test]
async fn test_array_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_array_length_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 3, max_length = 3)]
        ratings: [i32; 3],
    }

    Book::clear().await?;

    let book = Book::default().ratings([3, 4, 5]);

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_vecdeque_length_violation
#[tokio::test]
async fn test_vecdeque_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_vecdeque_length_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        editions: VecDeque<i32>,
    }

    Book::clear().await?;

    let too_short = Book::default().editions(VecDeque::from([1])); // ❌ below min

    let err = too_short.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 2"));

    Book::clear().await?;

    let too_long = Book::default().editions(VecDeque::from([1, 2, 3, 4])); // ❌ above max

    let err = too_long.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 3"));

    Ok(())
}

// Run test: cargo nextest run test_vecdeque_length_valid
#[tokio::test]
async fn test_vecdeque_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_vecdeque_length_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        editions: VecDeque<i32>,
    }

    Book::clear().await?;

    let book = Book::default().editions(VecDeque::from([1, 2]));

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_hashset_length_violation
#[tokio::test]
async fn test_hashset_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_hashset_length_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        tags: HashSet<String>,
    }

    Book::clear().await?;

    let too_short = Book::default().tags(HashSet::from(["rust".to_string()])); // ❌ below min

    let err = too_short.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 2"));

    Book::clear().await?;

    let too_long = Book::default().tags(HashSet::from([
        "rust".to_string(),
        "mongodb".to_string(),
        "odm".to_string(),
        "serde".to_string(), // ❌ above max
    ]));

    let err = too_long.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 3"));

    Ok(())
}

// Run test: cargo nextest run test_hashset_length_valid
#[tokio::test]
async fn test_hashset_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_hashset_length_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        tags: HashSet<String>,
    }

    Book::clear().await?;

    let book = Book::default().tags(HashSet::from(["rust".to_string(), "mongodb".to_string()]));

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_btreeset_length_violation
#[tokio::test]
async fn test_btreeset_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_btreeset_length_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        tags: BTreeSet<String>,
    }

    Book::clear().await?;

    let too_short = Book::default().tags(BTreeSet::from(["rust".to_string()])); // ❌ below min

    let err = too_short.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 2"));

    Book::clear().await?;

    let too_long = Book::default().tags(BTreeSet::from([
        "rust".to_string(),
        "mongodb".to_string(),
        "odm".to_string(),
        "serde".to_string(), // ❌ above max
    ]));

    let err = too_long.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 3"));

    Ok(())
}

// Run test: cargo nextest run test_btreeset_length_valid
#[tokio::test]
async fn test_btreeset_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_btreeset_length_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        tags: BTreeSet<String>,
    }

    Book::clear().await?;

    let book = Book::default().tags(BTreeSet::from(["rust".to_string(), "mongodb".to_string()]));

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_hashmap_length_violation
#[tokio::test]
async fn test_hashmap_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_hashmap_length_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        metadata: HashMap<String, String>,
    }

    Book::clear().await?;

    let too_short =
        Book::default().metadata(HashMap::from([("lang".to_string(), "en".to_string())])); // ❌ below min

    let err = too_short.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 2"));

    Book::clear().await?;

    let too_long = Book::default().metadata(HashMap::from([
        ("lang".to_string(), "en".to_string()),
        ("format".to_string(), "ebook".to_string()),
        ("edition".to_string(), "second".to_string()),
        ("region".to_string(), "ca".to_string()), // ❌ above max
    ]));

    let err = too_long.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 3"));

    Ok(())
}

// Run test: cargo nextest run test_hashmap_length_valid
#[tokio::test]
async fn test_hashmap_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_hashmap_length_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        metadata: HashMap<String, String>,
    }

    Book::clear().await?;

    let book = Book::default().metadata(HashMap::from([
        ("lang".to_string(), "en".to_string()),
        ("format".to_string(), "ebook".to_string()),
    ]));

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}

// Run test: cargo nextest run test_btreemap_length_violation
#[tokio::test]
async fn test_btreemap_length_violation() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_btreemap_length_violation")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        metadata: BTreeMap<String, String>,
    }

    Book::clear().await?;

    let too_short =
        Book::default().metadata(BTreeMap::from([("lang".to_string(), "en".to_string())])); // ❌ below min

    let err = too_short.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("must have a length of at least 2"));

    Book::clear().await?;

    let too_long = Book::default().metadata(BTreeMap::from([
        ("lang".to_string(), "en".to_string()),
        ("format".to_string(), "ebook".to_string()),
        ("edition".to_string(), "second".to_string()),
        ("region".to_string(), "ca".to_string()), // ❌ above max
    ]));

    let err = too_long.save().await;
    assert!(err.is_err());
    assert!(format!("{:?}", err).contains("at most 3"));

    Ok(())
}

// Run test: cargo nextest run test_btreemap_length_valid
#[tokio::test]
async fn test_btreemap_length_valid() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("validate_btreemap_length_valid")]
    struct Book {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(min_length = 2, max_length = 3)]
        metadata: BTreeMap<String, String>,
    }

    Book::clear().await?;

    let book = Book::default().metadata(BTreeMap::from([
        ("lang".to_string(), "en".to_string()),
        ("format".to_string(), "ebook".to_string()),
    ]));

    let result = book.save().await?;
    assert_ne!(result, ObjectId::default());

    Ok(())
}
