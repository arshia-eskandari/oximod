mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::{Model, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

// Run test:
// cargo nextest run typed_query_nested_document_field
#[tokio::test]
async fn typed_query_nested_document_field() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
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

    User::new()
        .name("User1")
        .address(Address::new().city("City1").active(true))
        .save()
        .await?;

    User::new()
        .name("User2")
        .address(Address::new().city("City2").active(true))
        .save()
        .await?;

    User::new()
        .name("User3")
        .address(Address::new().city("City1").active(false))
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

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
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

    User::new()
        .display_name("User1")
        .home_address(Address::new().city_name("City1").primary(true))
        .save()
        .await?;

    User::new()
        .display_name("User2")
        .home_address(Address::new().city_name("City2").primary(true))
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

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
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

    User::new()
        .name("User1")
        .address(Address::new().city("City1").active(true))
        .save()
        .await?;

    User::new()
        .name("User2")
        .address(Address::new().city("City2").active(true))
        .save()
        .await?;

    User::new().name("User3").save().await?;

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
// cargo nextest run typed_query_embedded_model_array_elem_match
#[tokio::test]
async fn typed_query_embedded_model_array_elem_match() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
    pub struct Address {
        city: String,
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_embedded_model_array_elem_match")]
    pub struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        addresses: Vec<Address>,
    }

    User::clear().await?;

    User::new()
        .name("User1")
        .addresses(vec![
            Address::new().city("City1").active(true),
            Address::new().city("City2").active(false),
        ])
        .save()
        .await?;

    User::new()
        .name("User2")
        .addresses(vec![
            Address::new().city("City1").active(false),
            Address::new().city("City2").active(true),
        ])
        .save()
        .await?;

    User::new()
        .name("User3")
        .addresses(vec![Address::new().city("City2").active(true)])
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

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
    pub struct Address {
        city: String,
        active: bool,
    }

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
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

    User::new()
        .name("User1")
        .profile(Profile::new().address(Address::new().city("City1").active(true)))
        .save()
        .await?;

    User::new()
        .name("User2")
        .profile(Profile::new().address(Address::new().city("City1").active(false)))
        .save()
        .await?;

    User::new()
        .name("User3")
        .profile(Profile::new().address(Address::new().city("City2").active(true)))
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

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
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

    User::new()
        .name("User1")
        .address(Address::new().city("City2"))
        .save()
        .await?;

    User::new()
        .name("User2")
        .address(Address::new().city("City3"))
        .save()
        .await?;

    User::new()
        .name("User3")
        .address(Address::new().city("City1"))
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

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
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

    User::new()
        .name("User1")
        .addresses(vec![Address::new().city_name("City1").active(true)])
        .save()
        .await?;

    User::new()
        .name("User2")
        .addresses(vec![Address::new().city_name("City1").active(false)])
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
// cargo nextest run typed_query_optional_nested_document_respects_serde_renames
#[tokio::test]
async fn typed_query_optional_nested_document_respects_serde_renames() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[model(embedded)]
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

    User::new()
        .name("User1")
        .address(Address::new().city_name("City1").active(true))
        .save()
        .await?;

    User::new()
        .name("User2")
        .address(Address::new().city_name("City1").active(false))
        .save()
        .await?;

    User::new().name("User3").save().await?;

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
