//! Integration tests for typed field-level query operators.
//!
//! The tests cover Serde field names, existence and null checks, regular
//! expressions, string helpers, negation, BSON types, modulo, and bitwise predicates.

mod common;

use common::init;
use mongodb::bson::{DateTime, oid::ObjectId};
use oximod::{BsonType, Model, Queryable, RegexOption};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

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

// Run test:
// cargo nextest run typed_query_matches_all_clear_bits
#[tokio::test]
async fn typed_query_matches_all_clear_bits() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_matches_all_clear_bits")]
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
        .filter(|user| user.permissions.bits_all_clear(0b1100))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User3");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_ordered_comparison_on_optional_numeric_field
#[tokio::test]
async fn typed_query_ordered_comparison_on_optional_numeric_field() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_ordered_comparison_on_optional_numeric_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        // No skip_serializing_if: None is stored as explicit BSON null.
        expires_at_ms: Option<i64>,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .expires_at_ms(1_000_i64)
        .save()
        .await?;

    User::default()
        .name("User2")
        .expires_at_ms(3_000_i64)
        .save()
        .await?;

    User::default().name("User3").save().await?;

    // The operand is the inner i64 value, not Option<i64>. The stored null
    // follows MongoDB's normal ordered-comparison semantics and never matches.
    let users: Vec<User> = User::query()
        .filter(|user| user.expires_at_ms.gt(2_000_i64))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User2");

    let users: Vec<User> = User::query()
        .filter(|user| user.expires_at_ms.lte(1_000_i64))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    let users: Vec<User> = User::query()
        .filter(|user| user.expires_at_ms.gte(1_000_i64) & user.expires_at_ms.lt(4_000_i64))
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
// cargo nextest run typed_query_ordered_comparison_on_optional_datetime_field
#[tokio::test]
async fn typed_query_ordered_comparison_on_optional_datetime_field() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_ordered_comparison_on_optional_datetime_field")]
    pub struct Session {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<DateTime>,
    }

    Session::clear().await?;

    Session::default()
        .name("Session1")
        .expires_at(DateTime::from_millis(1_000))
        .save()
        .await?;

    Session::default()
        .name("Session2")
        .expires_at(DateTime::from_millis(3_000))
        .save()
        .await?;

    Session::default().name("Session3").save().await?;

    // The operand is the inner DateTime value. The missing field follows
    // MongoDB's normal ordered-comparison semantics and never matches.
    let sessions: Vec<Session> = Session::query()
        .filter(|session| session.expires_at.gte(DateTime::from_millis(2_500)))
        .all()
        .await?;

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "Session2");

    let sessions: Vec<Session> = Session::query()
        .filter(|session| session.expires_at.lt(DateTime::from_millis(2_500)))
        .all()
        .await?;

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "Session1");

    Session::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_matches_numeric_modulo_on_optional_field
#[tokio::test]
async fn typed_query_matches_numeric_modulo_on_optional_field() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_matches_numeric_modulo_on_optional_field")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        login_count: Option<i32>,
    }

    User::clear().await?;

    User::default().name("User1").login_count(4).save().await?;

    User::default().name("User2").login_count(5).save().await?;

    User::default().name("User3").save().await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.login_count.modulo(2, 0))
        .all()
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "User1");

    User::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_matches_any_clear_bits
#[tokio::test]
async fn typed_query_matches_any_clear_bits() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_matches_any_clear_bits")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        permissions: i32,
    }

    User::clear().await?;

    User::default()
        .name("User1")
        .permissions(0b1100)
        .save()
        .await?;

    User::default()
        .name("User2")
        .permissions(0b1000)
        .save()
        .await?;

    User::default()
        .name("User3")
        .permissions(0b0010)
        .save()
        .await?;

    let users: Vec<User> = User::query()
        .filter(|user| user.permissions.bits_any_clear(0b1100))
        .sort_by(|user| user.name.asc())
        .all()
        .await?;

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "User2");
    assert_eq!(users[1].name, "User3");

    User::clear().await?;

    Ok(())
}
