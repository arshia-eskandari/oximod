//! Apply OxiMod's built-in validation rules to application data.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example validate_usage
//! ```
//!
//! This example demonstrates how to:
//!
//! - validate string lengths and content;
//! - validate email addresses and regular-expression patterns;
//! - require prefixes, suffixes, and contained text;
//! - restrict strings to ASCII alphanumeric characters;
//! - apply inclusive and exclusive numeric bounds;
//! - require positive or non-negative numbers;
//! - validate integer multiples;
//! - validate collection lengths;
//! - apply rules only when an optional value is present;
//! - require an optional field with `required`;
//! - validate explicitly before persistence;
//! - rely on `save()` to validate automatically;
//! - collect several field failures in one error.
//!
//! Validation runs when `validate()` is called and before save operations.
//! Raw MongoDB updates and typed update expressions do not automatically
//! validate the resulting stored document.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient, OxiModError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum ListingCategory {
    Office,
    Home,
}

// A marketplace listing stored as an independent MongoDB document.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("validation_product_listings")]
struct ProductListing {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    // Several compatible string rules can be combined on one field.
    //
    // A valid code looks like `ITEM-2026-0042-CA`.
    #[validate(
        pattern = r"^ITEM-2026-[0-9]{4}-CA$",
        starts_with = "ITEM-",
        ends_with = "-CA",
        includes = "2026"
    )]
    listing_code: String,

    // `alphanumeric` accepts ASCII letters and digits only.
    #[validate(min_length = 4, max_length = 12, alphanumeric)]
    seller_code: String,

    #[validate(min_length = 5, max_length = 60, non_empty)]
    title: String,

    #[validate(email)]
    contact_email: String,

    // Rules on an optional field run only when the option contains a value.
    // `None` would be accepted because this field is not marked `required`.
    #[validate(min_length = 10, max_length = 200, non_empty)]
    description: Option<String>,

    // `min_exclusive` changes the minimum from `>= 0.0` to `> 0.0`.
    #[validate(min = 0.0, min_exclusive)]
    price: f64,

    #[validate(non_negative)]
    stock: i32,

    // Integer fields can combine bounds with `multiple_of`.
    #[validate(min = 5, max = 100, multiple_of = 5)]
    case_quantity: u32,

    // Length validation also works with sequential collections.
    #[validate(min_length = 1, max_length = 4, non_empty)]
    tags: Vec<String>,

    // `required` applies to `Option<T>` and rejects `None`.
    #[validate(required)]
    category: Option<ListingCategory>,

    #[default(true)]
    active: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // This collection belongs only to the example, so clearing it keeps
    // repeated runs deterministic.
    ProductListing::clear().await?;

    println!("Valid listing");
    println!("-------------");

    let valid_listing = ProductListing::new()
        .listing_code("ITEM-2026-0042-CA")
        .seller_code("Seller1")
        .title("Ergonomic Desk Lamp")
        .contact_email("seller1@example.com")
        .description("Adjustable LED lamp for home and office use.")
        .price(49.99)
        .stock(24)
        .case_quantity(10_u32)
        .tags(vec!["office".to_string(), "lighting".to_string()])
        .category(ListingCategory::Office);

    // Explicit validation is useful before beginning a larger application
    // workflow. It performs no database operation.
    valid_listing.validate()?;

    println!("Preflight validation passed.");

    // `save()` validates again before inserting the document.
    let valid_id = valid_listing.save().await?;

    println!("Saved valid listing with _id: {valid_id}");

    println!();
    println!("Invalid listing");
    println!("---------------");

    let invalid_listing = ProductListing::new()
        // This value fails the pattern, prefix, suffix, and inclusion rules.
        .listing_code("LEGACY-0042-US")
        // The hyphen violates `alphanumeric`.
        .seller_code("seller-1")
        // This title is shorter than five characters.
        .title("Pen")
        .contact_email("not-an-email")
        // A present optional string is still validated.
        .description("   ")
        // The minimum is exclusive, so zero is invalid.
        .price(0.0)
        .stock(-2)
        // Twelve is within the bounds but is not a multiple of five.
        .case_quantity(12_u32)
        // An empty vector fails both the length and non-empty rules.
        .tags(Vec::new());

    match invalid_listing.validate() {
        Ok(()) => {
            println!("The invalid listing unexpectedly passed.");
        }
        Err(error) => {
            print_validation_failures(&error);
        }
    }

    // `save()` performs the same validation automatically and stops before
    // insertion if any rule fails.
    match invalid_listing.save().await {
        Ok(id) => {
            println!("Unexpectedly saved invalid listing with _id: {id}");
        }
        Err(error) => {
            let error_count = error
                .validation_errors()
                .map_or(0, <[oximod::ValidationError]>::len);

            println!(
                "Automatic save validation rejected \
                 {error_count} field-level failures."
            );
        }
    }

    println!();
    println!("Corrected listing");
    println!("-----------------");

    let corrected_listing = ProductListing::new()
        .listing_code("ITEM-2026-0088-CA")
        .seller_code("Seller2")
        .title("Cotton Storage Basket")
        .contact_email("seller2@example.com")
        // Leaving an optional field unset skips its non-required rules.
        .price(34.5)
        .stock(8)
        .case_quantity(20_u32)
        .tags(vec!["home".to_string(), "storage".to_string()])
        .category(ListingCategory::Home);

    let corrected_id = corrected_listing.save().await?;

    println!("Saved corrected listing with _id: {corrected_id}");

    // The invalid listing was rejected before insertion.
    let stored_count = ProductListing::count(doc! {}).await?;

    println!("Stored listings after the workflow: {stored_count}");

    Ok(())
}

fn print_validation_failures(error: &OxiModError) {
    let Some(errors) = error.validation_errors() else {
        println!("Validation returned a different error: {error}");
        return;
    };

    println!("OxiMod collected {} field-level failures:", errors.len());

    for validation_error in errors {
        println!("  {}: {}", validation_error.field, validation_error.message);
    }
}
