mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::BsonType;
use oximod::{
    BulkWriteOperation, EmbeddedDocument, Model, OxiModError, QueryError, QueryModifier, Queryable,
    RegexOption,
};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

// Run test:
// cargo nextest run typed_query_returns_only_matching_models
#[tokio::test]
async fn typed_query_returns_only_matching_models() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_returns_only_matching_models")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
        role: String,
    }

    User::clear().await?;

    // Confirm that #[derive(Model)] generated the expected typed fields.
    let fields = <User as Queryable>::fields();

    assert_eq!(fields._id.name(), "_id");
    assert_eq!(fields.name.name(), "name");
    assert_eq!(fields.age.name(), "age");
    assert_eq!(fields.active.name(), "active");
    assert_eq!(fields.role.name(), "role");

    User::default()
        .name("User1")
        .age(24)
        .active(true)
        .role("member")
        .save()
        .await?;

    User::default()
        .name("User2")
        .age(17)
        .active(true)
        .role("admin")
        .save()
        .await?;

    User::default()
        .name("User3")
        .age(30)
        .active(false)
        .role("admin")
        .save()
        .await?;

    User::default()
        .name("User4")
        .age(30)
        .active(true)
        .role("member")
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| {
            user.active.eq(true)
                & user.age.gte(18)
                & (user.role.eq("admin") | user.name.eq("User1"))
        })
        .all()
        .await?;

    assert_eq!(users.len(), 1);

    let user = &users[0];

    assert!(user._id.is_some());
    assert_eq!(user.name, "User1");
    assert_eq!(user.age, 24);
    assert!(user.active);
    assert_eq!(user.role, "member");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_first_returns_matching_model
#[tokio::test]
async fn typed_query_first_returns_matching_model() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_first_returns_matching_model")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .age(24)
        .active(true)
        .save()
        .await?;

    User::default()
        .name("User2")
        .age(30)
        .active(false)
        .save()
        .await?;

    let user: Option<User> = User::query()
        .filter(|user| user.name.eq("User1"))
        .first()
        .await?;

    let user = user.expect("User1 should be found");

    assert!(user._id.is_some());
    assert_eq!(user.name, "User1");
    assert_eq!(user.age, 24);
    assert!(user.active);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_first_returns_none_when_no_model_matches
#[tokio::test]
async fn typed_query_first_returns_none_when_no_model_matches() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_first_returns_none_when_no_model_matches")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
    }

    User::clear().await?;

    User::default().name("User1").age(24).save().await?;

    let user: Option<User> = User::query()
        .filter(|user| user.name.eq("MissingUser"))
        .first()
        .await?;

    assert!(user.is_none());

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_count_returns_number_of_matching_models
#[tokio::test]
async fn typed_query_count_returns_number_of_matching_models() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_count_returns_number_of_matching_models")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
        active: bool,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .age(24)
        .active(true)
        .save()
        .await?;

    User::default()
        .name("User2")
        .age(30)
        .active(true)
        .save()
        .await?;

    User::default()
        .name("User3")
        .age(17)
        .active(true)
        .save()
        .await?;

    User::default()
        .name("User4")
        .age(35)
        .active(false)
        .save()
        .await?;

    let count: u64 = User::query()
        .filter(|user| user.active.eq(true) & user.age.gte(18))
        .count()
        .await?;

    assert_eq!(count, 2);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_limit_restricts_number_of_results
#[tokio::test]
async fn typed_query_limit_restricts_number_of_results() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_limit_restricts_number_of_results")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        active: bool,
    }

    User::clear().await?;

    User::default().name("User1").active(true).save().await?;

    User::default().name("User2").active(true).save().await?;

    User::default().name("User3").active(true).save().await?;

    User::default().name("User4").active(false).save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.active.eq(true))
        .limit(2)
        .all()
        .await?;

    assert_eq!(users.len(), 2);
    assert!(users.iter().all(|user| user.active));

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_skip_omits_matching_results
#[tokio::test]
async fn typed_query_skip_omits_matching_results() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_skip_omits_matching_results")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        active: bool,
    }

    User::clear().await?;

    User::default().name("User1").active(true).save().await?;

    User::default().name("User2").active(true).save().await?;

    User::default().name("User3").active(true).save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.active.eq(true))
        .sort_by(|user| user.name.asc())
        .skip(1)
        .all()
        .await?;

    let names = users
        .iter()
        .map(|user| user.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["User2", "User3"]);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_sort_by_orders_results_ascending
#[tokio::test]
async fn typed_query_sort_by_orders_results_ascending() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_sort_by_orders_results_ascending")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
    }

    User::clear().await?;

    User::default().name("User1").age(30).save().await?;

    User::default().name("User2").age(18).save().await?;

    User::default().name("User3").age(24).save().await?;

    let users: Vec<User> = User::query().sort_by(|user| user.age.asc()).all().await?;

    let names = users
        .iter()
        .map(|user| user.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["User2", "User3", "User1",]);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_then_sort_by_orders_equal_values
#[tokio::test]
async fn typed_query_then_sort_by_orders_equal_values() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_then_sort_by_orders_equal_values")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
    }

    User::clear().await?;

    User::default().name("User3").age(24).save().await?;

    User::default().name("User1").age(18).save().await?;

    User::default().name("User2").age(24).save().await?;

    User::default().name("User4").age(30).save().await?;

    let users: Vec<User> = User::query()
        .sort_by(|user| user.age.asc())
        .then_sort_by(|user| user.name.asc())
        .all()
        .await?;

    let names = users
        .iter()
        .map(|user| user.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["User1", "User2", "User3", "User4",]);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_page_returns_requested_page
#[tokio::test]
async fn typed_query_page_returns_requested_page() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_page_returns_requested_page")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        age: i32,
    }

    User::clear().await?;

    User::default().name("User5").age(50).save().await?;

    User::default().name("User2").age(20).save().await?;

    User::default().name("User4").age(40).save().await?;

    User::default().name("User1").age(10).save().await?;

    User::default().name("User3").age(30).save().await?;

    let users: Vec<User> = User::query()
        .sort_by(|user| user.age.asc())
        .page(2, 2)
        .all()
        .await?;

    let ages = users.iter().map(|user| user.age).collect::<Vec<_>>();

    assert_eq!(ages, vec![30, 40]);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_in_values_matches_any_value
#[tokio::test]
async fn typed_query_in_values_matches_any_value() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_in_values_matches_any_value")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        role: String,
    }

    User::clear().await?;

    User::default().name("User1").role("admin").save().await?;

    User::default()
        .name("User2")
        .role("moderator")
        .save()
        .await?;

    User::default().name("User3").role("member").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.role.in_values(["admin", "moderator"]))
        .sort_by(|user| user.name.asc())
        .all()
        .await?;

    let names = users
        .iter()
        .map(|user| user.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["User1", "User2"]);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_not_in_values_excludes_values
#[tokio::test]
async fn typed_query_not_in_values_excludes_values() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_not_in_values_excludes_values")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
        role: String,
    }

    User::clear().await?;

    User::default().name("User1").role("admin").save().await?;

    User::default()
        .name("User2")
        .role("moderator")
        .save()
        .await?;

    User::default().name("User3").role("member").save().await?;

    User::default().name("User4").role("guest").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.role.not_in_values(["admin", "moderator"]))
        .sort_by(|user| user.name.asc())
        .all()
        .await?;

    let names = users
        .iter()
        .map(|user| user.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["User3", "User4"]);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_page_returns_error_for_zero_page_number
#[tokio::test]
async fn typed_query_page_returns_error_for_zero_page_number() {
    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_page_returns_error_for_zero_page_number")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        name: String,
    }

    let error = User::query()
        .page(0, 10)
        .all()
        .await
        .expect_err("page zero should return an error");

    assert!(matches!(
        error,
        OxiModError::Query(QueryError::InvalidPageNumber { page: 0 })
    ));
}

// Run test:
// cargo nextest run typed_query_uses_serde_renamed_field
#[tokio::test]
async fn typed_query_uses_serde_renamed_field() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_uses_serde_renamed_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[serde(rename = "displayName")]
        name: String,
    }

    User::clear().await?;

    let fields = <User as Queryable>::fields();

    assert_eq!(fields.name.name(), "displayName",);

    User::default().name("User1").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.name.eq("User1"))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_uses_serde_rename_all
#[tokio::test]
async fn typed_query_uses_serde_rename_all() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    #[db("test")]
    #[collection("typed_query_uses_serde_rename_all")]
    pub struct User {
        #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,
        display_name: String,
        account_active: bool,
    }

    User::clear().await?;

    let fields = <User as Queryable>::fields();

    assert_eq!(fields._id.name(), "_id");
    assert_eq!(fields.display_name.name(), "displayName");
    assert_eq!(fields.account_active.name(), "accountActive");

    User::default()
        .display_name("User1")
        .account_active(true)
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.display_name.eq("User1") & user.account_active.eq(true))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].display_name, "User1");
    assert!(users[0].account_active);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_exists_matches_present_fields
#[tokio::test]
async fn typed_query_exists_matches_present_fields() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_exists_matches_present_fields")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .nickname("Ace".to_owned())
        .save()
        .await?;

    User::default().name("User2").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.nickname.exists())
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");
    assert_eq!(users[0].nickname.as_deref(), Some("Ace"));

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_is_null_matches_explicit_null

#[tokio::test]
async fn typed_query_is_null_matches_explicit_null() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_is_null_matches_explicit_null")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        // No skip_serializing_if: None is stored as explicit BSON null.
        nickname: Option<String>,
    }

    User::clear().await?;

    User::default().name("User1").save().await?;

    User::default()
        .name("User2")
        .nickname("Ace".to_owned())
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.nickname.is_null())
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");
    assert_eq!(users[0].nickname, None);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_matches_regex
#[tokio::test]
async fn typed_query_matches_regex() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_matches_regex")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
    }

    User::clear().await?;

    User::default().name("User1").save().await?;
    User::default().name("User2").save().await?;
    User::default().name("Admin1").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.name.matches_regex("^User"))
        .sort_by(|user| user.name.asc())
        .all()
        .await?;

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "User1");
    assert_eq!(users[1].name, "User2");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_matches_regex_with_options
#[tokio::test]
async fn typed_query_matches_regex_with_options() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_matches_regex_with_options")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
    }

    User::clear().await?;

    User::default().name("User1").save().await?;
    User::default().name("USER2").save().await?;
    User::default().name("Admin1").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| {
            user.name
                .matches_regex_with_options("^user", [RegexOption::CaseInsensitive])
        })
        .sort_by(|user| user.name.asc())
        .all()
        .await?;

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "USER2");
    assert_eq!(users[1].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_array_contains_element
#[tokio::test]
async fn typed_query_array_contains_element() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_array_contains_element")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        tags: Vec<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .tags(vec!["rust".to_owned(), "mongodb".to_owned()])
        .save()
        .await?;

    User::default()
        .name("User2")
        .tags(vec!["typescript".to_owned()])
        .save()
        .await?;

    User::default()
        .name("User3")
        .tags(vec!["rust".to_owned(), "backend".to_owned()])
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.tags.contains("rust"))
        .sort_by(|user| user.name.asc())
        .all()
        .await?;

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "User1");
    assert_eq!(users[1].name, "User3");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_array_contains_all_elements
#[tokio::test]
async fn typed_query_array_contains_all_elements() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_array_contains_all_elements")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        tags: Vec<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .tags(vec![
            "rust".to_owned(),
            "mongodb".to_owned(),
            "backend".to_owned(),
        ])
        .save()
        .await?;

    User::default()
        .name("User2")
        .tags(vec!["rust".to_owned()])
        .save()
        .await?;

    User::default()
        .name("User3")
        .tags(vec!["mongodb".to_owned(), "typescript".to_owned()])
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.tags.contains_all(["rust", "mongodb"]))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_array_has_size
#[tokio::test]
async fn typed_query_array_has_size() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_array_has_size")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        tags: Vec<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .tags(vec!["rust".to_owned(), "mongodb".to_owned()])
        .save()
        .await?;

    User::default()
        .name("User2")
        .tags(vec!["rust".to_owned()])
        .save()
        .await?;

    User::default()
        .name("User3")
        .tags(vec![
            "rust".to_owned(),
            "mongodb".to_owned(),
            "backend".to_owned(),
        ])
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.tags.has_size(2))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_string_starts_with
#[tokio::test]
async fn typed_query_string_starts_with() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_string_starts_with")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
    }

    User::clear().await?;

    User::default().name("User.Admin").save().await?;

    User::default().name("UserXAdmin").save().await?;

    User::default().name("Admin.User").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.name.starts_with("User."))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User.Admin");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_string_ends_with
#[tokio::test]
async fn typed_query_string_ends_with() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_string_ends_with")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
    }

    User::clear().await?;

    User::default().name("Admin.User").save().await?;

    User::default().name("AdminXUser").save().await?;

    User::default().name("User.Admin").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.name.ends_with(".User"))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Admin.User");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_string_contains_text
#[tokio::test]
async fn typed_query_string_contains_text() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_string_contains_text")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
    }

    User::clear().await?;

    User::default().name("Primary.User.Admin").save().await?;

    User::default().name("Primary.UserXAdmin").save().await?;

    User::default().name("Primary.Admin").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.name.contains_text("User."))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Primary.User.Admin");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_field_not_negates_comparison
#[tokio::test]
async fn typed_query_field_not_negates_comparison() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_field_not_negates_comparison")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        age: i32,
    }

    User::clear().await?;

    User::default().name("User1").age(17).save().await?;

    User::default().name("User2").age(18).save().await?;

    User::default().name("User3").age(25).save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.age.not(|age| age.gte(18)))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_array_elem_match
#[tokio::test]
async fn typed_query_array_elem_match() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_array_elem_match")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        scores: Vec<i32>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .scores(vec![75, 85, 95])
        .save()
        .await?;

    User::default()
        .name("User2")
        .scores(vec![70, 95])
        .save()
        .await?;

    User::default()
        .name("User3")
        .scores(vec![79, 90])
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.scores.elem_match(|score| score.gte(80) & score.lt(90)))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_nested_document_field
#[tokio::test]
async fn typed_query_nested_document_field() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, PartialEq, Default)]
    pub struct Address {
        city: String,
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_nested_document_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        address: Address,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .address(Address {
            city: "City1".to_owned(),
            active: true,
        })
        .save()
        .await?;

    User::default()
        .name("User2")
        .address(Address {
            city: "City2".to_owned(),
            active: true,
        })
        .save()
        .await?;

    User::default()
        .name("User3")
        .address(Address {
            city: "City1".to_owned(),
            active: false,
        })
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| {
            user.address
                .nested(|address| address.city.eq("City1") & address.active.eq(true))
        })
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_nested_document_respects_serde_renames
#[tokio::test]
async fn typed_query_nested_document_respects_serde_renames() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, PartialEq, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct Address {
        city_name: String,

        #[serde(rename = "isPrimary")]
        primary: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    #[db("test")]
    #[collection("typed_query_nested_document_respects_serde_renames")]
    pub struct User {
        #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        display_name: String,
        home_address: Address,
    }

    User::clear().await?;

    User::default()
        .display_name("User1")
        .home_address(Address {
            city_name: "City1".to_owned(),
            primary: true,
        })
        .save()
        .await?;

    User::default()
        .display_name("User2")
        .home_address(Address {
            city_name: "City2".to_owned(),
            primary: true,
        })
        .save()
        .await?;

    let fields = <User as Queryable>::fields();

    assert_eq!(fields.home_address.name(), "homeAddress");

    let nested_paths = fields.home_address.nested(|address| {
        (
            address.city_name.name().to_owned(),
            address.primary.name().to_owned(),
        )
    });

    assert_eq!(
        nested_paths,
        (
            "homeAddress.cityName".to_owned(),
            "homeAddress.isPrimary".to_owned(),
        )
    );

    let users: Vec<User> = User::query()
        .filter(|user| {
            user.home_address
                .nested(|address| address.city_name.eq("City1") & address.primary.eq(true))
        })
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].display_name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_optional_nested_document
#[tokio::test]
async fn typed_query_optional_nested_document() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, PartialEq, Default)]
    pub struct Address {
        city: String,
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_optional_nested_document")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        address: Option<Address>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .address(Address {
            city: "City1".to_owned(),
            active: true,
        })
        .save()
        .await?;

    User::default()
        .name("User2")
        .address(Address {
            city: "City2".to_owned(),
            active: true,
        })
        .save()
        .await?;

    User::default().name("User3").save().await?;

    let fields = <User as Queryable>::fields();

    let paths = fields.address.nested(|address| {
        (
            address.city.name().to_owned(),
            address.active.name().to_owned(),
        )
    });

    assert_eq!(
        paths,
        ("address.city".to_owned(), "address.active".to_owned(),)
    );

    let users: Vec<User> = User::query()
        .filter(|user| {
            user.address
                .nested(|address| address.city.eq("City1") & address.active.eq(true))
        })
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_embedded_document_array_elem_match
#[tokio::test]
async fn typed_query_embedded_document_array_elem_match() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, PartialEq, Default)]
    pub struct Address {
        city: String,
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_embedded_document_array_elem_match")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        addresses: Vec<Address>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .addresses(vec![
            Address {
                city: "City1".to_owned(),
                active: true,
            },
            Address {
                city: "City2".to_owned(),
                active: false,
            },
        ])
        .save()
        .await?;

    User::default()
        .name("User2")
        .addresses(vec![
            Address {
                city: "City1".to_owned(),
                active: false,
            },
            Address {
                city: "City2".to_owned(),
                active: true,
            },
        ])
        .save()
        .await?;

    User::default()
        .name("User3")
        .addresses(vec![Address {
            city: "City2".to_owned(),
            active: true,
        }])
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| {
            user.addresses
                .elem_match_nested(|address| address.city.eq("City1") & address.active.eq(true))
        })
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_supports_multiple_nested_document_levels
#[tokio::test]
async fn typed_query_supports_multiple_nested_document_levels() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    pub struct Address {
        city: String,
        active: bool,
    }

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, PartialEq, Default)]
    pub struct Profile {
        address: Address,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_supports_multiple_nested_document_levels")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        profile: Profile,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .profile(Profile {
            address: Address {
                city: "City1".to_owned(),
                active: true,
            },
        })
        .save()
        .await?;

    User::default()
        .name("User2")
        .profile(Profile {
            address: Address {
                city: "City1".to_owned(),
                active: false,
            },
        })
        .save()
        .await?;

    User::default()
        .name("User3")
        .profile(Profile {
            address: Address {
                city: "City2".to_owned(),
                active: true,
            },
        })
        .save()
        .await?;

    let fields = <User as Queryable>::fields();

    let paths = fields.profile.nested(|profile| {
        profile.address.nested(|address| {
            (
                address.city.name().to_owned(),
                address.active.name().to_owned(),
            )
        })
    });

    assert_eq!(
        paths,
        (
            "profile.address.city".to_owned(),
            "profile.address.active".to_owned(),
        )
    );

    let users: Vec<User> = User::query()
        .filter(|user| {
            user.profile.nested(|profile| {
                profile
                    .address
                    .nested(|address| address.city.eq("City1") & address.active.eq(true))
            })
        })
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_sorts_by_nested_document_field
#[tokio::test]
async fn typed_query_sorts_by_nested_document_field() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    pub struct Address {
        city: String,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_sorts_by_nested_document_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        address: Address,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .address(Address {
            city: "City2".to_owned(),
        })
        .save()
        .await?;

    User::default()
        .name("User2")
        .address(Address {
            city: "Kitchener".to_owned(),
        })
        .save()
        .await?;

    User::default()
        .name("User3")
        .address(Address {
            city: "City1".to_owned(),
        })
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .sort_by(|user| user.address.nested(|address| address.city.asc()))
        .all()
        .await?;

    assert_eq!(users.len(), 3);
    assert_eq!(users[0].name, "User3");
    assert_eq!(users[1].name, "User1");
    assert_eq!(users[2].name, "User2");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_nested_elem_match_respects_serde_renames
#[tokio::test]
async fn typed_query_nested_elem_match_respects_serde_renames() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Address {
        city_name: String,

        #[serde(rename = "isActive")]
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_nested_elem_match_respects_serde_renames")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        addresses: Vec<Address>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .addresses(vec![Address {
            city_name: "City1".to_owned(),
            active: true,
        }])
        .save()
        .await?;

    User::default()
        .name("User2")
        .addresses(vec![Address {
            city_name: "City1".to_owned(),
            active: false,
        }])
        .save()
        .await?;

    let fields = <User as Queryable>::fields();

    let paths = fields.addresses.elem_match_nested(|address| {
        assert_eq!(address.city_name.name(), "cityName");
        assert_eq!(address.active.name(), "isActive");

        address.city_name.eq("City1") & address.active.eq(true)
    });

    let users: Vec<User> = User::query().filter(|_| paths).all().await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_optional_string_contains_text
#[tokio::test]
async fn typed_query_optional_string_contains_text() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_optional_string_contains_text")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .nickname("cool_user".to_owned())
        .save()
        .await?;

    User::default()
        .name("User2")
        .nickname("regular_user".to_owned())
        .save()
        .await?;

    User::default().name("User3").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.nickname.contains_text("cool_"))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_optional_nested_document_respects_serde_renames
#[tokio::test]
async fn typed_query_optional_nested_document_respects_serde_renames() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Address {
        city_name: String,

        #[serde(rename = "isActive")]
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_optional_nested_document_respects_serde_renames")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        address: Option<Address>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .address(Address {
            city_name: "City1".to_owned(),
            active: true,
        })
        .save()
        .await?;

    User::default()
        .name("User2")
        .address(Address {
            city_name: "City1".to_owned(),
            active: false,
        })
        .save()
        .await?;

    User::default().name("User3").save().await?;

    let fields = <User as Queryable>::fields();

    fields.address.nested(|address| {
        assert_eq!(address.city_name.name(), "address.cityName");
        assert_eq!(address.active.name(), "address.isActive");
    });

    let users: Vec<User> = User::query()
        .filter(|user| {
            user.address
                .nested(|address| address.city_name.eq("City1") & address.active.eq(true))
        })
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_deletes_one_matching_document
#[tokio::test]
async fn typed_query_deletes_one_matching_document() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_deletes_one_matching_document")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        active: bool,
    }

    User::clear().await?;

    User::default().name("User1").active(false).save().await?;

    User::default().name("User2").active(false).save().await?;

    User::default().name("User3").active(true).save().await?;

    let deleted_user = User::query()
        .filter(|user| user.active.eq(false))
        .sort_by(|user| user.name.asc())
        .delete_one()
        .await?
        .expect("one inactive user should be deleted");

    assert_eq!(deleted_user.name, "User1");
    assert!(!deleted_user.active);

    let remaining_users: Vec<User> = User::query().sort_by(|user| user.name.asc()).all().await?;

    assert_eq!(remaining_users.len(), 2);
    assert_eq!(remaining_users[0].name, "User2");
    assert_eq!(remaining_users[1].name, "User3");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_deletes_all_matching_documents
#[tokio::test]
async fn typed_query_deletes_all_matching_documents() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_deletes_all_matching_documents")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        active: bool,
    }

    User::clear().await?;

    User::default().name("User1").active(false).save().await?;

    User::default().name("User2").active(false).save().await?;

    User::default().name("User3").active(true).save().await?;

    let result = User::query()
        .filter(|user| user.active.eq(false))
        .delete_all()
        .await?;

    assert_eq!(result.deleted_count, 2);

    let remaining_users: Vec<User> = User::query().sort_by(|user| user.name.asc()).all().await?;

    assert_eq!(remaining_users.len(), 1);
    assert_eq!(remaining_users[0].name, "User3");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_updates_one_field_with_set
#[tokio::test]
async fn typed_query_updates_one_field_with_set() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_updates_one_field_with_set")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        active: bool,
    }

    User::clear().await?;

    User::default().name("User1").active(false).save().await?;

    User::default().name("User2").active(false).save().await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.active.set(true))
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");
    assert!(updated_user.active);

    let user = User::query()
        .filter(|user| user.name.eq("User1"))
        .first()
        .await?
        .expect("updated user should still exist");

    assert!(user.active);

    let unchanged_user = User::query()
        .filter(|user| user.name.eq("User2"))
        .first()
        .await?
        .expect("second user should still exist");

    assert!(!unchanged_user.active);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_updates_multiple_fields_with_set
#[tokio::test]
async fn typed_query_updates_multiple_fields_with_set() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_updates_multiple_fields_with_set")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        active: bool,
        age: i32,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .active(false)
        .age(20)
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.name.set("UpdatedUser") & user.active.set(true) & user.age.set(21))
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "UpdatedUser");
    assert!(updated_user.active);
    assert_eq!(updated_user.age, 21);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_unsets_optional_field
#[tokio::test]
async fn typed_query_unsets_optional_field() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_unsets_optional_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .nickname("cool_user".to_owned())
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.nickname.unset())
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");
    assert_eq!(updated_user.nickname, None);

    let users_without_nickname: Vec<User> = User::query()
        .filter(|user| user.nickname.not_exists())
        .all()
        .await?;

    assert_eq!(users_without_nickname.len(), 1);
    assert_eq!(users_without_nickname[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_increments_numeric_field
#[tokio::test]
async fn typed_query_increments_numeric_field() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_increments_numeric_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        login_count: i32,
        balance: f64,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .login_count(5)
        .balance(10.5)
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.login_count.inc(2) & user.balance.inc(1.5))
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");
    assert_eq!(updated_user.login_count, 7);
    assert_eq!(updated_user.balance, 12.0);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_updates_all_matching_documents
#[tokio::test]
async fn typed_query_updates_all_matching_documents() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_updates_all_matching_documents")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        active: bool,
        login_count: i32,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .active(false)
        .login_count(1)
        .save()
        .await?;

    User::default()
        .name("User2")
        .active(false)
        .login_count(2)
        .save()
        .await?;

    User::default()
        .name("User3")
        .active(true)
        .login_count(3)
        .save()
        .await?;

    let result = User::query()
        .filter(|user| user.active.eq(false))
        .update_all(|user| user.active.set(true) & user.login_count.inc(1))
        .await?;

    assert_eq!(result.matched_count, 2);
    assert_eq!(result.modified_count, 2);

    let users: Vec<User> = User::query().sort_by(|user| user.name.asc()).all().await?;

    assert_eq!(users.len(), 3);

    assert_eq!(users[0].name, "User1");
    assert!(users[0].active);
    assert_eq!(users[0].login_count, 2);

    assert_eq!(users[1].name, "User2");
    assert!(users[1].active);
    assert_eq!(users[1].login_count, 3);

    assert_eq!(users[2].name, "User3");
    assert!(users[2].active);
    assert_eq!(users[2].login_count, 3);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_pushes_value_to_array
#[tokio::test]
async fn typed_query_pushes_value_to_array() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_pushes_value_to_array")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        tags: Vec<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .tags(vec!["rust".to_owned()])
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.tags.push("mongodb"))
        .await?
        .expect("one user should be updated");

    assert_eq!(
        updated_user.tags,
        vec!["rust".to_owned(), "mongodb".to_owned(),]
    );

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_adds_unique_value_to_array
#[tokio::test]
async fn typed_query_adds_unique_value_to_array() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_adds_unique_value_to_array")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        tags: Vec<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .tags(vec!["rust".to_owned()])
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.tags.add_to_set("mongodb"))
        .await?
        .expect("one user should be updated");

    assert_eq!(
        updated_user.tags,
        vec!["rust".to_owned(), "mongodb".to_owned(),]
    );

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.tags.add_to_set("mongodb"))
        .await?
        .expect("one user should be updated");

    assert_eq!(
        updated_user.tags,
        vec!["rust".to_owned(), "mongodb".to_owned(),]
    );

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_pulls_value_from_array
#[tokio::test]
async fn typed_query_pulls_value_from_array() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_pulls_value_from_array")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        tags: Vec<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .tags(vec![
            "rust".to_owned(),
            "mongodb".to_owned(),
            "backend".to_owned(),
        ])
        .save()
        .await?;

    User::default()
        .name("User2")
        .tags(vec!["rust".to_owned(), "mongodb".to_owned()])
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.tags.pull("mongodb"))
        .await?
        .expect("one user should be updated");

    assert_eq!(
        updated_user.tags,
        vec!["rust".to_owned(), "backend".to_owned(),]
    );

    let unchanged_user = User::query()
        .filter(|user| user.name.eq("User2"))
        .first()
        .await?
        .expect("second user should still exist");

    assert_eq!(
        unchanged_user.tags,
        vec!["rust".to_owned(), "mongodb".to_owned(),]
    );

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_pops_array_elements
#[tokio::test]
async fn typed_query_pops_array_elements() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_pops_array_elements")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        tags: Vec<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .tags(vec![
            "rust".to_owned(),
            "mongodb".to_owned(),
            "backend".to_owned(),
        ])
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.tags.pop_first())
        .await?
        .expect("one user should be updated");

    assert_eq!(
        updated_user.tags,
        vec!["mongodb".to_owned(), "backend".to_owned(),]
    );

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.tags.pop_last())
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.tags, vec!["mongodb".to_owned()]);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_pushes_multiple_values_to_array
#[tokio::test]
async fn typed_query_pushes_multiple_values_to_array() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_pushes_multiple_values_to_array")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        tags: Vec<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .tags(vec!["rust".to_owned()])
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.tags.push_each(["mongodb", "backend"]))
        .await?
        .expect("one user should be updated");

    assert_eq!(
        updated_user.tags,
        vec![
            "rust".to_owned(),
            "mongodb".to_owned(),
            "backend".to_owned(),
        ]
    );

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_adds_multiple_unique_values_to_array
#[tokio::test]
async fn typed_query_adds_multiple_unique_values_to_array() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_adds_multiple_unique_values_to_array")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        tags: Vec<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .tags(vec!["rust".to_owned(), "mongodb".to_owned()])
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.tags.add_each_to_set(["mongodb", "backend", "systems"]))
        .await?
        .expect("one user should be updated");

    assert_eq!(
        updated_user.tags,
        vec![
            "rust".to_owned(),
            "mongodb".to_owned(),
            "backend".to_owned(),
            "systems".to_owned(),
        ]
    );

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_sets_nested_document_field
#[tokio::test]
async fn typed_query_sets_nested_document_field() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Address {
        city_name: String,
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_sets_nested_document_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        address: Address,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .address(Address {
            city_name: "City1".to_owned(),
            active: true,
        })
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| {
            user.address
                .nested(|address| address.city_name.set("City2"))
        })
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");
    assert_eq!(updated_user.address.city_name, "City2");
    assert!(updated_user.address.active);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_unsets_optional_nested_document_field
#[tokio::test]
async fn typed_query_unsets_optional_nested_document_field() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Address {
        city_name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        postal_code: Option<String>,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_unsets_optional_nested_document_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        address: Address,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .address(Address {
            city_name: "City1".to_owned(),
            postal_code: Some("A1A 1A1".to_owned()),
        })
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.address.nested(|address| address.postal_code.unset()))
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");
    assert_eq!(updated_user.address.city_name, "City1");
    assert_eq!(updated_user.address.postal_code, None);

    let users_without_postal_code: Vec<User> = User::query()
        .filter(|user| {
            user.address
                .nested(|address| address.postal_code.not_exists())
        })
        .all()
        .await?;

    assert_eq!(users_without_postal_code.len(), 1);
    assert_eq!(users_without_postal_code[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_increments_nested_numeric_field
#[tokio::test]
async fn typed_query_increments_nested_numeric_field() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Statistics {
        login_count: i32,
        balance: f64,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_increments_nested_numeric_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        statistics: Statistics,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .statistics(Statistics {
            login_count: 5,
            balance: 10.5,
        })
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| {
            user.statistics
                .nested(|statistics| statistics.login_count.inc(2) & statistics.balance.inc(1.5))
        })
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");
    assert_eq!(updated_user.statistics.login_count, 7);
    assert_eq!(updated_user.statistics.balance, 12.0);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_updates_serde_renamed_fields
#[tokio::test]
async fn typed_query_updates_serde_renamed_fields() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    #[db("test")]
    #[collection("typed_query_updates_serde_renamed_fields")]
    pub struct User {
        #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        display_name: String,

        #[serde(rename = "isActive")]
        active: bool,

        login_count: i32,
    }

    User::clear().await?;

    User::default()
        .display_name("User1")
        .active(false)
        .login_count(5)
        .save()
        .await?;

    let fields = <User as Queryable>::fields();

    assert_eq!(fields.display_name.name(), "displayName");
    assert_eq!(fields.active.name(), "isActive");
    assert_eq!(fields.login_count.name(), "loginCount");

    let updated_user = User::query()
        .filter(|user| user.display_name.eq("User1"))
        .update_one(|user| {
            user.display_name.set("UpdatedUser") & user.active.set(true) & user.login_count.inc(2)
        })
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.display_name, "UpdatedUser");
    assert!(updated_user.active);
    assert_eq!(updated_user.login_count, 7);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_updates_multi_level_nested_field
#[tokio::test]
async fn typed_query_updates_multi_level_nested_field() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Address {
        city_name: String,
        active: bool,
    }

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Profile {
        address: Address,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_updates_multi_level_nested_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        profile: Profile,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .profile(Profile {
            address: Address {
                city_name: "City1".to_owned(),
                active: false,
            },
        })
        .save()
        .await?;

    let fields = <User as Queryable>::fields();

    let paths = fields.profile.nested(|profile| {
        profile.address.nested(|address| {
            (
                address.city_name.name().to_owned(),
                address.active.name().to_owned(),
            )
        })
    });

    assert_eq!(
        paths,
        (
            "profile.address.cityName".to_owned(),
            "profile.address.active".to_owned(),
        )
    );

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| {
            user.profile.nested(|profile| {
                profile
                    .address
                    .nested(|address| address.city_name.set("City2") & address.active.set(true))
            })
        })
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");
    assert_eq!(updated_user.profile.address.city_name, "City2");
    assert!(updated_user.profile.address.active);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_errors_propagate_through_write_operations
#[tokio::test]
async fn typed_query_errors_propagate_through_write_operations() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_errors_propagate_through_write_operations")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        active: bool,
    }

    let delete_one_error = User::query()
        .page(0, 10)
        .delete_one()
        .await
        .expect_err("delete_one should propagate the query error");

    assert!(matches!(
        delete_one_error,
        OxiModError::Query(QueryError::InvalidPageNumber { page: 0 })
    ));

    let delete_all_error = User::query()
        .page(0, 10)
        .delete_all()
        .await
        .expect_err("delete_all should propagate the query error");

    assert!(matches!(
        delete_all_error,
        OxiModError::Query(QueryError::InvalidPageNumber { page: 0 })
    ));

    let update_one_error = User::query()
        .page(0, 10)
        .update_one(|user| user.active.set(true))
        .await
        .expect_err("update_one should propagate the query error");

    assert!(matches!(
        update_one_error,
        OxiModError::Query(QueryError::InvalidPageNumber { page: 0 })
    ));

    let update_all_error = User::query()
        .page(0, 10)
        .update_all(|user| user.active.set(true))
        .await
        .expect_err("update_all should propagate the query error");

    assert!(matches!(
        update_all_error,
        OxiModError::Query(QueryError::InvalidPageNumber { page: 0 })
    ));

    Ok(())
}

// Run test:
// cargo nextest run typed_query_bulk_writes_reject_query_modifiers
#[tokio::test]
async fn typed_query_bulk_writes_reject_query_modifiers() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_bulk_writes_reject_query_modifiers")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        active: bool,
    }

    let delete_sort_error = User::query()
        .sort_by(|user| user.name.asc())
        .delete_all()
        .await
        .expect_err("delete_all should reject sorting");

    assert!(matches!(
        delete_sort_error,
        OxiModError::Query(QueryError::UnsupportedBulkWriteModifier {
            operation: BulkWriteOperation::DeleteAll,
            modifier: QueryModifier::Sort,
        })
    ));

    let delete_limit_error = User::query()
        .limit(1)
        .delete_all()
        .await
        .expect_err("delete_all should reject limits");

    assert!(matches!(
        delete_limit_error,
        OxiModError::Query(QueryError::UnsupportedBulkWriteModifier {
            operation: BulkWriteOperation::DeleteAll,
            modifier: QueryModifier::Limit,
        })
    ));

    let update_skip_error = User::query()
        .skip(1)
        .update_all(|user| user.active.set(true))
        .await
        .expect_err("update_all should reject offsets");

    assert!(matches!(
        update_skip_error,
        OxiModError::Query(QueryError::UnsupportedBulkWriteModifier {
            operation: BulkWriteOperation::UpdateAll,
            modifier: QueryModifier::Skip,
        })
    ));

    let update_page_error = User::query()
        .page(2, 10)
        .update_all(|user| user.active.set(true))
        .await
        .expect_err("update_all should reject pagination");

    assert!(matches!(
        update_page_error,
        OxiModError::Query(QueryError::UnsupportedBulkWriteModifier {
            operation: BulkWriteOperation::UpdateAll,
            modifier: QueryModifier::Pagination,
        })
    ));

    Ok(())
}

// Run test:
// cargo nextest run typed_query_multiplies_numeric_fields
#[tokio::test]
async fn typed_query_multiplies_numeric_fields() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_multiplies_numeric_fields")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        score: i32,
        balance: f64,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .score(10)
        .balance(12.5)
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.score.mul(3) & user.balance.mul(2.0))
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");
    assert_eq!(updated_user.score, 30);
    assert_eq!(updated_user.balance, 25.0);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_applies_minimum_numeric_values
#[tokio::test]
async fn typed_query_applies_minimum_numeric_values() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_applies_minimum_numeric_values")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        score: i32,
        balance: f64,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .score(10)
        .balance(12.5)
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.score.min(8) & user.balance.min(20.0))
        .await?
        .expect("one user should be updated");

    // 8 is lower than the stored score, so it replaces 10.
    assert_eq!(updated_user.score, 8);

    // 20.0 is higher than the stored balance, so 12.5 remains.
    assert_eq!(updated_user.balance, 12.5);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_applies_maximum_numeric_values
#[tokio::test]
async fn typed_query_applies_maximum_numeric_values() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_applies_maximum_numeric_values")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        score: i32,
        balance: f64,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .score(10)
        .balance(12.5)
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.score.max(15) & user.balance.max(10.0))
        .await?
        .expect("one user should be updated");

    // 15 is higher than the stored score, so it replaces 10.
    assert_eq!(updated_user.score, 15);

    // 10.0 is lower than the stored balance, so 12.5 remains.
    assert_eq!(updated_user.balance, 12.5);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_renames_optional_field
#[tokio::test]
async fn typed_query_renames_optional_field() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_renames_optional_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,

        #[serde(rename = "displayAlias", skip_serializing_if = "Option::is_none")]
        display_alias: Option<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .nickname("cool_user".to_owned())
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.nickname.rename_to(&user.display_alias))
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");
    assert_eq!(updated_user.nickname, None);
    assert_eq!(updated_user.display_alias, Some("cool_user".to_owned()));

    let fields = <User as Queryable>::fields();

    assert_eq!(fields.nickname.name(), "nickname");
    assert_eq!(fields.display_alias.name(), "displayAlias");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_sets_field_to_current_date
#[tokio::test]
async fn typed_query_sets_field_to_current_date() -> TestResult {
    init().await?;

    use mongodb::bson::DateTime;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_sets_field_to_current_date")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        updated_at: Option<DateTime>,
    }

    User::clear().await?;

    let original_date = DateTime::from_millis(0);

    User::default()
        .name("User1")
        .updated_at(original_date)
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .update_one(|user| user.updated_at.current_date())
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.name, "User1");

    let updated_at = updated_user
        .updated_at
        .expect("updated_at should contain the current date");

    assert!(updated_at > original_date);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_updates_first_matching_array_element
#[tokio::test]
async fn typed_query_updates_first_matching_array_element() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct Address {
        city_name: String,
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_updates_first_matching_array_element")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        addresses: Vec<Address>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .addresses(vec![
            Address {
                city_name: "City1".to_owned(),
                active: false,
            },
            Address {
                city_name: "City1".to_owned(),
                active: false,
            },
            Address {
                city_name: "City2".to_owned(),
                active: false,
            },
        ])
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| {
            user.addresses
                .elem_match_nested(|address| address.city_name.eq("City1"))
        })
        .update_one(|user| {
            user.addresses
                .positional(|address| address.active.set(true))
        })
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.addresses.len(), 3);

    assert_eq!(updated_user.addresses[0].city_name, "City1");
    assert!(updated_user.addresses[0].active);

    assert_eq!(updated_user.addresses[1].city_name, "City1");
    assert!(!updated_user.addresses[1].active);

    assert_eq!(updated_user.addresses[2].city_name, "City2");
    assert!(!updated_user.addresses[2].active);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_updates_array_elements_matching_filter
#[tokio::test]
async fn typed_query_updates_array_elements_matching_filter() -> TestResult {
    init().await?;

    #[derive(EmbeddedDocument, Serialize, Deserialize, Debug, Default, PartialEq)]
    pub struct Address {
        city: String,
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_updates_array_elements_matching_filter")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        addresses: Vec<Address>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .addresses(vec![
            Address {
                city: "City1".to_owned(),
                active: false,
            },
            Address {
                city: "City1".to_owned(),
                active: false,
            },
            Address {
                city: "City2".to_owned(),
                active: false,
            },
        ])
        .save()
        .await?;

    let updated_user = User::query()
        .filter(|user| user.name.eq("User1"))
        .array_filter(|user| {
            user.addresses
                .array_filter("address", |address| address.city.eq("City1"))
        })
        .update_one(|user| {
            user.addresses
                .filtered("address", |address| address.active.set(true))
        })
        .await?
        .expect("one user should be updated");

    assert_eq!(updated_user.addresses.len(), 3);

    assert_eq!(updated_user.addresses[0].city, "City1");
    assert!(updated_user.addresses[0].active);

    assert_eq!(updated_user.addresses[1].city, "City1");
    assert!(updated_user.addresses[1].active);

    assert_eq!(updated_user.addresses[2].city, "City2");
    assert!(!updated_user.addresses[2].active);

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_matches_bson_field_type
#[tokio::test]
async fn typed_query_matches_bson_field_type() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_matches_bson_field_type")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .nickname("cool_user".to_owned())
        .save()
        .await?;

    User::default().name("User2").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.nickname.has_bson_type(BsonType::String))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_matches_numeric_modulo
#[tokio::test]
async fn typed_query_matches_numeric_modulo() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_matches_numeric_modulo")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        login_count: i32,
    }

    User::clear().await?;

    User::default().name("User1").login_count(4).save().await?;

    User::default().name("User2").login_count(5).save().await?;

    User::default().name("User3").login_count(6).save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.login_count.modulo(2, 0))
        .sort_by(|user| user.name.asc())
        .all()
        .await?;

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "User1");
    assert_eq!(users[1].name, "User3");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_matches_all_set_bits
#[tokio::test]
async fn typed_query_matches_all_set_bits() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_matches_all_set_bits")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        permissions: i32,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .permissions(0b0111)
        .save()
        .await?;

    User::default()
        .name("User2")
        .permissions(0b0101)
        .save()
        .await?;

    User::default()
        .name("User3")
        .permissions(0b0010)
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.permissions.bits_all_set(0b0101))
        .sort_by(|user| user.name.asc())
        .all()
        .await?;

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "User1");
    assert_eq!(users[1].name, "User2");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_matches_any_set_bits
#[tokio::test]
async fn typed_query_matches_any_set_bits() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_matches_any_set_bits")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        permissions: i32,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .permissions(0b1000)
        .save()
        .await?;

    User::default()
        .name("User2")
        .permissions(0b0100)
        .save()
        .await?;

    User::default()
        .name("User3")
        .permissions(0b0010)
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.permissions.bits_any_set(0b1100))
        .sort_by(|user| user.name.asc())
        .all()
        .await?;

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "User1");
    assert_eq!(users[1].name, "User2");

    User::clear().await?;

    Ok(())
}
