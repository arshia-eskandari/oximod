mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::{EmbeddedDocument, Model, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

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
