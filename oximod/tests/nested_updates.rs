mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::{EmbeddedDocument, Model, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

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
