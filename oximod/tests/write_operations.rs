//! Integration tests for typed query write operations.
//!
//! The tests cover single and bulk deletes and updates, field-update operators,
//! returned models and counts, query-error propagation, and rejected bulk modifiers.

mod common;

use common::init;
use mongodb::bson::{DateTime, oid::ObjectId};
use oximod::{BulkWriteOperation, Model, OxiModError, QueryError, QueryModifier, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

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
