//! Typed-query usage example for the oximod crate
//!
//! Run with: `cargo run --example typed_query`
//!
//! This demonstrates how to:
//! - Build type-safe queries with `Queryable`
//! - Use equality, inequality, and ordered comparisons
//! - Combine filters using `&` and `|`
//! - Negate a field comparison with `not`
//! - Match or exclude multiple values
//! - Sort by one or more fields
//! - Apply limits, offsets, and one-based pagination
//! - Retrieve all, first, and count results
//! - Distinguish missing fields from explicit BSON null
//! - Match strings with regular expressions and typed regex options
//! - Perform escaped prefix, suffix, and substring matching
//! - Query arrays with `contains`, `contains_all`, and `has_size`
//! - Use `$elemMatch` with scalar arrays
//! - Query fields inside optional embedded documents
//! - Sort by nested document fields
//! - Use `$elemMatch` with arrays of embedded documents
//! - Respect Serde field renaming in generated query paths

use mongodb::bson::oid::ObjectId;
use oximod::{EmbeddedDocument, Model, OxiClient, Queryable, RegexOption};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, EmbeddedDocument, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Address {
    city_name: String,
    active: bool,

    #[serde(rename = "isPrimary")]
    primary: bool,
}

#[derive(Debug, Serialize, Deserialize, Model)]
#[serde(rename_all = "camelCase")]
#[db("typed_query_example")]
#[collection("users")]
struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    display_name: String,
    age: i32,
    active: bool,

    // `None` is omitted from MongoDB, so this field can demonstrate
    // the distinction between missing and present fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    nickname: Option<String>,

    // `None` is serialized as BSON null because it is not skipped.
    middle_name: Option<String>,

    tags: Vec<String>,
    scores: Vec<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<Address>,

    addresses: Vec<Address>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")?;

    OxiClient::init_global(mongodb_uri).await?;

    User::clear().await?;

    seed_users().await?;

    basic_comparisons().await?;
    logical_expressions().await?;
    membership_queries().await?;
    sorting_and_pagination().await?;
    execution_methods().await?;
    missing_and_null_queries().await?;
    string_queries().await?;
    array_queries().await?;
    nested_document_queries().await?;

    User::clear().await?;

    Ok(())
}

async fn seed_users() -> Result<(), Box<dyn std::error::Error>> {
    User::default()
        .display_name("User1")
        .age(30)
        .active(true)
        .nickname("cool_user".to_owned())
        .middle_name("A".to_owned())
        .tags(vec!["rust".to_owned(), "mongodb".to_owned()])
        .scores(vec![75, 85, 95])
        .address(Address {
            city_name: "City1".to_owned(),
            active: true,
            primary: true,
        })
        .addresses(vec![
            Address {
                city_name: "City1".to_owned(),
                active: true,
                primary: true,
            },
            Address {
                city_name: "City2".to_owned(),
                active: false,
                primary: false,
            },
        ])
        .save()
        .await?;

    User::default()
        .display_name("User2")
        .age(17)
        .active(false)
        .tags(vec!["typescript".to_owned()])
        .scores(vec![70, 95])
        .address(Address {
            city_name: "City2".to_owned(),
            active: true,
            primary: true,
        })
        .addresses(vec![
            Address {
                city_name: "City1".to_owned(),
                active: false,
                primary: false,
            },
            Address {
                city_name: "City2".to_owned(),
                active: true,
                primary: true,
            },
        ])
        .save()
        .await?;

    User::default()
        .display_name("User3")
        .age(22)
        .active(true)
        .tags(vec!["rust".to_owned(), "backend".to_owned()])
        .scores(vec![79, 90])
        .addresses(vec![Address {
            city_name: "City2".to_owned(),
            active: true,
            primary: true,
        }])
        .save()
        .await?;

    Ok(())
}

async fn basic_comparisons() -> Result<(), Box<dyn std::error::Error>> {
    let adults = User::query()
        .filter(|user| {
            user.active.eq(true)
                & user.age.gte(18)
                & user.age.lt(65)
                & user.display_name.ne("User2")
        })
        .sort_by(|user| user.age.desc())
        .then_sort_by(|user| user.display_name.asc())
        .all()
        .await?;

    println!("Active adults: {adults:#?}");

    let minors = User::query()
        .filter(|user| user.age.not(|age| age.gte(18)))
        .all()
        .await?;

    println!("Users younger than 18: {minors:#?}");

    Ok(())
}

async fn logical_expressions() -> Result<(), Box<dyn std::error::Error>> {
    let users = User::query()
        .filter(|user| {
            user.active.eq(true) & (user.tags.contains("rust") | user.display_name.eq("User2"))
        })
        .all()
        .await?;

    println!("Combined AND/OR query: {users:#?}");

    Ok(())
}

async fn membership_queries() -> Result<(), Box<dyn std::error::Error>> {
    let selected_users = User::query()
        .filter(|user| user.display_name.in_values(["User1", "User3"]))
        .all()
        .await?;

    println!("Selected users: {selected_users:#?}");

    let remaining_users = User::query()
        .filter(|user| user.display_name.not_in_values(["User1", "User3"]))
        .all()
        .await?;

    println!("Users not selected: {remaining_users:#?}");

    Ok(())
}

async fn sorting_and_pagination() -> Result<(), Box<dyn std::error::Error>> {
    let sorted_users = User::query()
        .sort_by(|user| user.age.desc())
        .then_sort_by(|user| user.display_name.asc())
        .all()
        .await?;

    println!("Sorted users: {sorted_users:#?}");

    let limited_users = User::query()
        .sort_by(|user| user.display_name.asc())
        .skip(1)
        .limit(1)
        .all()
        .await?;

    println!("Skipped and limited users: {limited_users:#?}");

    // Pagination is one-based.
    let first_page = User::query()
        .sort_by(|user| user.display_name.asc())
        .page(1, 2)
        .all()
        .await?;

    println!("First page: {first_page:#?}");

    Ok(())
}

async fn execution_methods() -> Result<(), Box<dyn std::error::Error>> {
    let first_active_user = User::query()
        .filter(|user| user.active.eq(true))
        .sort_by(|user| user.display_name.asc())
        .first()
        .await?;

    println!("First active user: {first_active_user:#?}");

    let active_user_count = User::query()
        .filter(|user| user.active.eq(true))
        .count()
        .await?;

    println!("Active-user count: {active_user_count}");

    Ok(())
}

async fn missing_and_null_queries() -> Result<(), Box<dyn std::error::Error>> {
    let users_with_nicknames = User::query()
        .filter(|user| user.nickname.exists())
        .all()
        .await?;

    println!(
        "Users whose nickname field exists: \
         {users_with_nicknames:#?}"
    );

    let users_without_nicknames = User::query()
        .filter(|user| user.nickname.not_exists())
        .all()
        .await?;

    println!(
        "Users whose nickname field is missing: \
         {users_without_nicknames:#?}"
    );

    let non_null_nicknames = User::query()
        .filter(|user| user.nickname.is_not_null())
        .all()
        .await?;

    println!(
        "Users with non-null nicknames: \
         {non_null_nicknames:#?}"
    );

    let null_middle_names = User::query()
        .filter(|user| user.middle_name.is_null())
        .all()
        .await?;

    println!(
        "Users with an explicit null middle name: \
         {null_middle_names:#?}"
    );

    Ok(())
}

async fn string_queries() -> Result<(), Box<dyn std::error::Error>> {
    let regex_matches = User::query()
        .filter(|user| {
            user.display_name
                .matches_regex_with_options("^user", [RegexOption::CaseInsensitive])
        })
        .sort_by(|user| user.display_name.asc())
        .all()
        .await?;

    println!("Case-insensitive regex matches: {regex_matches:#?}");

    let prefix_matches = User::query()
        .filter(|user| user.display_name.starts_with("User"))
        .all()
        .await?;

    println!("Prefix matches: {prefix_matches:#?}");

    let suffix_matches = User::query()
        .filter(|user| user.display_name.ends_with("1"))
        .all()
        .await?;

    println!("Suffix matches: {suffix_matches:#?}");

    let substring_matches = User::query()
        .filter(|user| user.nickname.contains_text("cool_"))
        .all()
        .await?;

    println!("Substring matches: {substring_matches:#?}");

    Ok(())
}

async fn array_queries() -> Result<(), Box<dyn std::error::Error>> {
    let rust_users = User::query()
        .filter(|user| user.tags.contains("rust"))
        .all()
        .await?;

    println!("Users with the rust tag: {rust_users:#?}");

    let rust_and_mongodb_users = User::query()
        .filter(|user| user.tags.contains_all(["rust", "mongodb"]))
        .all()
        .await?;

    println!(
        "Users with rust and mongodb tags: \
         {rust_and_mongodb_users:#?}"
    );

    let users_with_two_tags = User::query()
        .filter(|user| user.tags.has_size(2))
        .all()
        .await?;

    println!("Users with exactly two tags: {users_with_two_tags:#?}");

    let users_with_score_in_range = User::query()
        .filter(|user| user.scores.elem_match(|score| score.gte(80) & score.lt(90)))
        .all()
        .await?;

    println!(
        "Users with a score from 80 through 89: \
         {users_with_score_in_range:#?}"
    );

    Ok(())
}

async fn nested_document_queries() -> Result<(), Box<dyn std::error::Error>> {
    let city1_users = User::query()
        .filter(|user| {
            user.address
                .nested(|address| address.city_name.eq("City1") & address.active.eq(true))
        })
        .all()
        .await?;

    println!(
        "Users with an active City1 address: \
         {city1_users:#?}"
    );

    let users_sorted_by_city = User::query()
        .filter(|user| user.address.exists())
        .sort_by(|user| user.address.nested(|address| address.city_name.asc()))
        .all()
        .await?;

    println!(
        "Users sorted by nested city: \
         {users_sorted_by_city:#?}"
    );

    let users_with_matching_address = User::query()
        .filter(|user| {
            user.addresses.elem_match_nested(|address| {
                address.city_name.eq("City1") & address.active.eq(true) & address.primary.eq(true)
            })
        })
        .all()
        .await?;

    println!(
        "Users with a matching embedded address: \
         {users_with_matching_address:#?}"
    );

    Ok(())
}
