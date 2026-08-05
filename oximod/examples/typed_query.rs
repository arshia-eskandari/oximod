//! Build type-safe MongoDB queries with OxiMod.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example typed_query
//! ```
//!
//! This example demonstrates how to:
//!
//! - compare typed fields without writing BSON field names;
//! - combine expressions with `&`, `|`, and `not()`;
//! - compose a query through repeated `filter()` calls;
//! - match or exclude several values;
//! - sort by multiple fields;
//! - use limits, offsets, and one-based pagination;
//! - execute queries with `all()`, `first()`, and `count()`;
//! - distinguish missing optional fields from BSON null;
//! - use typed string and regular-expression helpers;
//! - query scalar arrays;
//! - query optional embedded documents;
//! - use `$elemMatch` with arrays of embedded models;
//! - preserve Serde-renamed MongoDB paths automatically.
//!
//! Unlike raw BSON queries, incompatible operations fail at compile time. For
//! example, numeric comparisons cannot be applied to a boolean field and array
//! operators cannot be applied to a string field.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiClient, OxiModError, Queryable, RegexOption};
use serde::{Deserialize, Serialize};

// An assignment embedded inside a work item.
//
// Embedded models expose typed nested fields but do not own collections and
// cannot begin root queries themselves.
#[derive(Debug, Serialize, Deserialize, Model)]
#[model(embedded)]
#[serde(rename_all = "camelCase")]
struct Assignment {
    team_name: String,
    active: bool,

    #[serde(rename = "isPrimary")]
    primary: bool,
}

// A work item stored as an independent MongoDB document.
//
// Serde converts Rust snake_case names to camelCase in MongoDB. The generated
// typed fields use those serialized names automatically.
#[derive(Debug, Serialize, Deserialize, Model)]
#[serde(rename_all = "camelCase")]
#[db("oximod_examples")]
#[collection("typed_query_work_items")]
struct WorkItem {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    title: String,
    priority: i32,
    open: bool,

    // `None` is omitted, allowing `exists()` and `not_exists()` to demonstrate
    // genuinely missing MongoDB fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<String>,

    // `None` is serialized as BSON null because this field is not skipped.
    reviewer: Option<String>,

    labels: Vec<String>,
    estimates: Vec<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    assignment: Option<Assignment>,

    assignments: Vec<Assignment>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // This collection belongs only to the example, so clearing it keeps every
    // run deterministic.
    WorkItem::clear().await?;

    seed_work_items().await?;

    comparisons_and_logic().await?;
    composable_filters().await?;
    membership_queries().await?;
    sorting_and_pagination().await?;
    execution_methods().await?;
    missing_and_null_queries().await?;
    string_queries().await?;
    array_queries().await?;
    nested_queries().await?;

    Ok(())
}

/// Demonstrates typed comparisons and grouped logical expressions.
async fn comparisons_and_logic() -> Result<(), OxiModError> {
    println!("Comparisons and logical expressions");
    println!("-----------------------------------");

    let important_backend_work = WorkItem::query()
        .filter(|item| item.open.eq(true) & item.priority.gte(4) & item.labels.contains("backend"))
        .sort_by(|item| item.priority.desc())
        .then_sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles(
        "Open backend work with priority 4 or higher",
        &important_backend_work,
    );

    let urgent_or_bugged = WorkItem::query()
        .filter(|item| item.priority.gte(5) | item.labels.contains("bug"))
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles(
        "Priority-5 work or work labelled as a bug",
        &urgent_or_bugged,
    );

    let closed_items = WorkItem::query()
        .filter(|item| item.open.not(|open| open.eq(true)))
        .all()
        .await?;

    print_titles("Items that are not open", &closed_items);

    println!();

    Ok(())
}

/// Shows that repeated filters are combined using logical AND.
async fn composable_filters() -> Result<(), OxiModError> {
    println!("Composable filters");
    println!("------------------");

    let open_rust_work = WorkItem::query()
        .filter(|item| item.open.eq(true))
        .filter(|item| item.labels.contains("rust"))
        .sort_by(|item| item.priority.desc())
        .all()
        .await?;

    print_titles(
        "Open work that also carries the rust label",
        &open_rust_work,
    );

    println!();

    Ok(())
}

/// Demonstrates `$in` and `$nin` through typed field methods.
async fn membership_queries() -> Result<(), OxiModError> {
    println!("Membership queries");
    println!("------------------");

    let selected = WorkItem::query()
        .filter(|item| {
            item.title
                .in_values(["Build query API", "Write onboarding guide"])
        })
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles("Selected titles", &selected);

    let remaining = WorkItem::query()
        .filter(|item| {
            item.title
                .not_in_values(["Build query API", "Write onboarding guide"])
        })
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles("Titles outside the selection", &remaining);

    println!();

    Ok(())
}

/// Demonstrates deterministic ordering, offsets, limits, and pagination.
async fn sorting_and_pagination() -> Result<(), OxiModError> {
    println!("Sorting and pagination");
    println!("----------------------");

    let ordered = WorkItem::query()
        .sort_by(|item| item.priority.desc())
        .then_sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles_with_priority("All items by priority", &ordered);

    let middle_item = WorkItem::query()
        .sort_by(|item| item.priority.desc())
        .then_sort_by(|item| item.title.asc())
        .skip(1)
        .limit(1)
        .all()
        .await?;

    print_titles_with_priority("One item after skipping the highest priority", &middle_item);

    // Pagination is one-based. Page 2 with a size of 2 skips the first two
    // results and returns the next two.
    let second_page = WorkItem::query()
        .sort_by(|item| item.priority.desc())
        .then_sort_by(|item| item.title.asc())
        .page(2, 2)
        .all()
        .await?;

    print_titles_with_priority("Second page with two items per page", &second_page);

    println!();

    Ok(())
}

/// Compares the three principal typed read execution methods.
async fn execution_methods() -> Result<(), OxiModError> {
    println!("Execution methods");
    println!("-----------------");

    let first_open_item = WorkItem::query()
        .filter(|item| item.open.eq(true))
        .sort_by(|item| item.priority.desc())
        .then_sort_by(|item| item.title.asc())
        .first()
        .await?;

    match first_open_item {
        Some(item) => {
            println!("First open item by priority: {}", item.title);
        }
        None => {
            println!("First open item by priority: none");
        }
    }

    let open_count = WorkItem::query()
        .filter(|item| item.open.eq(true))
        .count()
        .await?;

    println!("Number of open items: {open_count}");
    println!();

    Ok(())
}

/// Contrasts an omitted field with a field serialized as BSON null.
async fn missing_and_null_queries() -> Result<(), OxiModError> {
    println!("Missing fields and BSON null");
    println!("----------------------------");

    let owned_items = WorkItem::query()
        .filter(|item| item.owner.exists())
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles("Items whose owner field exists", &owned_items);

    let unowned_items = WorkItem::query()
        .filter(|item| item.owner.not_exists())
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles("Items whose owner field is missing", &unowned_items);

    let awaiting_reviewers = WorkItem::query()
        .filter(|item| item.reviewer.is_null())
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles(
        "Items whose reviewer is stored as null",
        &awaiting_reviewers,
    );

    println!();

    Ok(())
}

/// Demonstrates escaped helpers and configurable regular expressions.
async fn string_queries() -> Result<(), OxiModError> {
    println!("String queries");
    println!("--------------");

    let build_or_fix = WorkItem::query()
        .filter(|item| {
            item.title
                .matches_regex_with_options("^(build|fix)", [RegexOption::CaseInsensitive])
        })
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles("Titles beginning with build or fix", &build_or_fix);

    let guide = WorkItem::query()
        .filter(|item| item.title.starts_with("Write"))
        .all()
        .await?;

    print_titles("Titles beginning with Write", &guide);

    let pagination = WorkItem::query()
        .filter(|item| item.title.contains_text("pagination"))
        .all()
        .await?;

    print_titles("Titles containing pagination", &pagination);

    println!();

    Ok(())
}

/// Demonstrates membership, `$all`, `$size`, and scalar `$elemMatch`.
async fn array_queries() -> Result<(), OxiModError> {
    println!("Array queries");
    println!("-------------");

    let backend_items = WorkItem::query()
        .filter(|item| item.labels.contains("backend"))
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles("Items carrying the backend label", &backend_items);

    let rust_backend_items = WorkItem::query()
        .filter(|item| item.labels.contains_all(["rust", "backend"]))
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles("Items carrying both rust and backend", &rust_backend_items);

    let single_label_items = WorkItem::query()
        .filter(|item| item.labels.has_size(1))
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles("Items containing exactly one label", &single_label_items);

    // `$elemMatch` guarantees that one array element satisfies both bounds.
    let medium_estimate_items = WorkItem::query()
        .filter(|item| {
            item.estimates
                .elem_match(|estimate| estimate.gte(4) & estimate.lte(6))
        })
        .sort_by(|item| item.title.asc())
        .all()
        .await?;

    print_titles(
        "Items containing an estimate from 4 through 6",
        &medium_estimate_items,
    );

    println!();

    Ok(())
}

/// Demonstrates typed paths through embedded documents.
async fn nested_queries() -> Result<(), OxiModError> {
    println!("Nested embedded-model queries");
    println!("-----------------------------");

    let team2_items = WorkItem::query()
        .filter(|item| {
            item.assignment
                .nested(|assignment| assignment.team_name.eq("Team2") & assignment.active.eq(true))
        })
        .all()
        .await?;

    print_titles("Items with an active Team2 assignment", &team2_items);

    let assigned_items = WorkItem::query()
        .filter(|item| item.assignment.exists())
        .sort_by(|item| {
            item.assignment
                .nested(|assignment| assignment.team_name.asc())
        })
        .all()
        .await?;

    println!("Items ordered by nested team name:");

    for item in &assigned_items {
        let team = item
            .assignment
            .as_ref()
            .map(|assignment| assignment.team_name.as_str())
            .unwrap_or("unassigned");

        println!("  {team:<6} {}", item.title);
    }

    let primary_team1_items = WorkItem::query()
        .filter(|item| {
            item.assignments.elem_match_nested(|assignment| {
                assignment.team_name.eq("Team1")
                    & assignment.active.eq(true)
                    & assignment.primary.eq(true)
            })
        })
        .all()
        .await?;

    print_titles(
        "Items with one active primary Team1 assignment",
        &primary_team1_items,
    );

    println!();

    Ok(())
}

/// Inserts a deterministic dataset used by every query section.
async fn seed_work_items() -> Result<(), OxiModError> {
    let items = [
        WorkItem::new()
            .title("Build query API")
            .priority(5)
            .open(true)
            .owner("User1")
            .reviewer("User2")
            .labels(vec!["rust".to_string(), "backend".to_string()])
            .estimates(vec![3, 5, 8])
            .assignment(
                Assignment::new()
                    .team_name("Team1")
                    .active(true)
                    .primary(true),
            )
            .assignments(vec![
                Assignment::new()
                    .team_name("Team1")
                    .active(true)
                    .primary(true),
                Assignment::new()
                    .team_name("Team2")
                    .active(false)
                    .primary(false),
            ]),
        WorkItem::new()
            .title("Write onboarding guide")
            .priority(3)
            .open(true)
            .labels(vec!["docs".to_string()])
            .estimates(vec![2, 3])
            .assignment(
                Assignment::new()
                    .team_name("Team2")
                    .active(true)
                    .primary(true),
            )
            .assignments(vec![
                Assignment::new()
                    .team_name("Team1")
                    .active(false)
                    .primary(false),
                Assignment::new()
                    .team_name("Team2")
                    .active(true)
                    .primary(true),
            ]),
        WorkItem::new()
            .title("Fix pagination edge case")
            .priority(4)
            .open(true)
            .owner("User3")
            .labels(vec![
                "rust".to_string(),
                "backend".to_string(),
                "bug".to_string(),
            ])
            .estimates(vec![1, 2, 5])
            .assignments(vec![
                Assignment::new()
                    .team_name("Team2")
                    .active(true)
                    .primary(true),
            ]),
        WorkItem::new()
            .title("Archive legacy report")
            .priority(1)
            .open(false)
            .reviewer("User4")
            .labels(vec!["maintenance".to_string()])
            .estimates(vec![1])
            .assignment(
                Assignment::new()
                    .team_name("Team3")
                    .active(false)
                    .primary(true),
            )
            .assignments(vec![
                Assignment::new()
                    .team_name("Team3")
                    .active(false)
                    .primary(true),
            ]),
    ];

    for item in items {
        item.save().await?;
    }

    Ok(())
}

fn print_titles(label: &str, items: &[WorkItem]) {
    let titles = items
        .iter()
        .map(|item| item.title.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    println!(
        "{label}: {}",
        if titles.is_empty() { "none" } else { &titles }
    );
}

fn print_titles_with_priority(label: &str, items: &[WorkItem]) {
    let titles = items
        .iter()
        .map(|item| format!("{} ({})", item.title, item.priority))
        .collect::<Vec<_>>()
        .join(", ");

    println!(
        "{label}: {}",
        if titles.is_empty() { "none" } else { &titles }
    );
}
