mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::{Model, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

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
