mod common;

use common::init;

use mongodb::bson::oid::ObjectId;
use oximod::{_query::Queryable, Model};
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

    /*
        MongoDB equivalent:

        {
            "$and": [
                { "active": true },
                { "age": { "$gte": 18 } },
                {
                    "$or": [
                        { "role": "admin" },
                        { "name": "User1" }
                    ]
                }
            ]
        }

        Expected results:

        User1:
        - active
        - adult
        - name matches

        User2:
        - excluded because age is below 18

        User3:
        - excluded because active is false

        User4:
        - excluded because neither role nor name matches
    */
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
