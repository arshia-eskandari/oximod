//! Typed-query usage example for the oximod crate
//!
//! Run with: `cargo run --example typed_query`
//!
//! This demonstrates how to:
//! - Build type-safe queries with `Queryable`
//! - Combine filters using `&` and `|`
//! - Use equality and ordered comparison operators
//! - Sort results by one or more fields
//! - Apply limits, offsets, and pagination
//! - Query optional fields for null or missing values
//! - Match strings with regular expressions and typed regex options
//! - Query array fields with `contains`, `contains_all`, and `has_size`
//! - Respect Serde field renaming in generated queries

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiClient, Queryable, RegexOption};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[serde(rename_all = "camelCase")]
#[db("typed_query_example")]
#[collection("users")]
struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    name: String,
    age: i32,
    active: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    nickname: Option<String>,

    tags: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    OxiClient::init_global(mongodb_uri).await?;

    User::clear().await?;

    User::default()
        .name("User1")
        .age(30)
        .active(true)
        .nickname("cool_user".to_owned())
        .tags(vec!["rust".to_owned(), "mongodb".to_owned()])
        .save()
        .await?;

    User::default()
        .name("User2")
        .age(17)
        .active(false)
        .tags(vec!["typescript".to_owned()])
        .save()
        .await?;

    let adults = User::query()
        .filter(|user| user.active.eq(true) & user.age.gte(18))
        .sort_by(|user| user.age.desc())
        .then_sort_by(|user| user.name.asc())
        .all()
        .await?;

    println!("Adults: {adults:#?}");

    let rust_users = User::query()
        .filter(|user| user.tags.contains("rust"))
        .all()
        .await?;

    println!("Rust users: {rust_users:#?}");

    let matching_names = User::query()
        .filter(|user| {
            user.name
                .matches_regex_with_options("^user1", [RegexOption::CaseInsensitive])
        })
        .all()
        .await?;

    println!("Matching names: {matching_names:#?}");

    let users_with_nicknames = User::query()
        .filter(|user| user.nickname.is_not_null())
        .all()
        .await?;

    println!("Users with nicknames: {users_with_nicknames:#?}");

    User::clear().await?;

    Ok(())
}
