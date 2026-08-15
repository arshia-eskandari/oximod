//! Integration tests proving `#[validate(nested)]` flows through every
//! modern OxiMod path that already performs whole-model validation:
//! explicit-client saves, session-aware saves, bulk-write preflight, and
//! save-time validation after mutable pre-save hooks. The final test pins
//! the intentional boundary: update expressions remain non-validating.
//!
//! These tests use an explicit `mongodb::Client` so they cannot collide
//! with tests that initialize the process-global client.

use mongodb::{Client, bson::doc, bson::oid::ObjectId};
use oximod::{Hooks, Model, OxiModError, Queryable};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

async fn client() -> Result<Client, Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
    Ok(Client::with_uri_str(uri).await?)
}

#[derive(Model, Serialize, Deserialize, Debug, Clone)]
#[model(embedded)]
struct Address {
    #[validate(non_empty)]
    city: String,
}

fn assert_nested_city_error(error: &OxiModError, expected_path: &str) {
    let errors = error
        .validation_errors()
        .unwrap_or_else(|| panic!("expected a Validation error, got {error:?}"));

    assert_eq!(errors.len(), 1, "expected exactly one error: {errors:?}");
    assert_eq!(errors[0].field, expected_path);
    assert_eq!(errors[0].message, "must be non-empty");
}

// Run test: cargo nextest run save_from_rejects_invalid_nested_child
#[tokio::test]
async fn save_from_rejects_invalid_nested_child() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_save_from")]
    struct Customer {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        address: Address,
    }

    let client = client().await?;
    Customer::clear_from(&client).await?;

    let error = Customer::new()
        .address(Address::new().city(""))
        .save_from(&client)
        .await
        .map(|_| ())
        .expect_err("save_from must reject an invalid nested child");

    assert_nested_city_error(&error, "address.city");

    assert_eq!(
        Customer::count_from(doc! {}, &client).await?,
        0,
        "no invalid document may be written"
    );

    Customer::new()
        .address(Address::new().city("Springfield"))
        .save_from(&client)
        .await?;

    assert_eq!(Customer::count_from(doc! {}, &client).await?, 1);

    Ok(())
}

// Run test: cargo nextest run save_from_mut_rejects_invalid_nested_child
#[tokio::test]
async fn save_from_mut_rejects_invalid_nested_child() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_save_from_mut")]
    struct Customer {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        address: Address,
    }

    let client = client().await?;
    Customer::clear_from(&client).await?;

    let mut customer = Customer::new().address(Address::new().city(""));

    let error = customer
        .save_from_mut(&client)
        .await
        .map(|_| ())
        .expect_err("save_from_mut must reject an invalid nested child");

    assert_nested_city_error(&error, "address.city");

    assert_eq!(Customer::count_from(doc! {}, &client).await?, 0);

    Ok(())
}

// Run test: cargo nextest run session_saves_reject_invalid_nested_child
#[tokio::test]
async fn session_saves_reject_invalid_nested_child() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_session_saves")]
    struct Customer {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        address: Address,
    }

    let client = client().await?;
    Customer::clear_from(&client).await?;

    let mut session = client.start_session().await?;
    session.start_transaction().await?;

    let error = Customer::new()
        .address(Address::new().city(""))
        .save_with_session(&mut session)
        .await
        .map(|_| ())
        .expect_err("save_with_session must reject an invalid nested child");

    assert_nested_city_error(&error, "address.city");

    let mut invalid = Customer::new().address(Address::new().city(""));

    let error = invalid
        .save_mut_with_session(&mut session)
        .await
        .map(|_| ())
        .expect_err("save_mut_with_session must reject an invalid nested child");

    assert_nested_city_error(&error, "address.city");

    // A valid document still saves inside the same transaction.
    Customer::new()
        .address(Address::new().city("Springfield"))
        .save_with_session(&mut session)
        .await?;

    session.commit_transaction().await?;

    assert_eq!(
        Customer::count_from(doc! {}, &client).await?,
        1,
        "only the valid document may be committed"
    );

    Ok(())
}

// Run test: cargo nextest run bulk_insert_preflight_rejects_invalid_nested_child
#[tokio::test]
async fn bulk_insert_preflight_rejects_invalid_nested_child() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_bulk_insert")]
    struct Customer {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        address: Address,
    }

    let client = client().await?;
    Customer::clear_from(&client).await?;

    let error = Customer::bulk_write()
        .insert(Customer::new().address(Address::new().city("Springfield")))
        .insert(Customer::new().address(Address::new().city("")))
        .insert(Customer::new().address(Address::new().city("Shelbyville")))
        .execute_from(&client)
        .await
        .map(|_| ())
        .expect_err("an invalid nested child must fail the bulk preflight");

    assert_nested_city_error(&error, "address.city");

    assert_eq!(
        Customer::count_from(doc! {}, &client).await?,
        0,
        "bulk preflight must prevent every queued write"
    );

    Ok(())
}

// Run test: cargo nextest run bulk_replace_preflight_rejects_invalid_nested_child
#[tokio::test]
async fn bulk_replace_preflight_rejects_invalid_nested_child() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_bulk_replace")]
    struct Customer {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[validate(nested)]
        address: Address,
    }

    let client = client().await?;
    Customer::clear_from(&client).await?;

    Customer::new()
        .name("original")
        .address(Address::new().city("Springfield"))
        .save_from(&client)
        .await?;

    let error = Customer::bulk_write()
        .replace_one(
            Customer::query().filter(|customer| customer.name.eq("original")),
            Customer::new()
                .name("replacement")
                .address(Address::new().city("")),
        )
        .execute_from(&client)
        .await
        .map(|_| ())
        .expect_err("an invalid nested replacement must fail the bulk preflight");

    assert_nested_city_error(&error, "address.city");

    assert_eq!(
        Customer::count_from(
            doc! { "name": "original", "address.city": "Springfield" },
            &client
        )
        .await?,
        1,
        "the stored document must remain unmodified"
    );

    Ok(())
}

// Run test: cargo nextest run pre_save_mut_cannot_bypass_nested_validation
#[tokio::test]
async fn pre_save_mut_cannot_bypass_nested_validation() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_hook_mutation")]
    #[hooks]
    struct Customer {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        address: Address,
    }

    #[async_trait::async_trait]
    impl Hooks for Customer {
        async fn pre_save_mut(&mut self) -> Result<(), OxiModError> {
            // Mutate the nested child into an invalid state; save-time
            // validation runs afterward and must catch it.
            self.address.city = String::new();
            Ok(())
        }
    }

    let client = client().await?;
    Customer::clear_from(&client).await?;

    let mut customer = Customer::new().address(Address::new().city("Springfield"));

    let error = customer
        .save_from_mut(&client)
        .await
        .map(|_| ())
        .expect_err("validation after pre_save_mut must catch the nested violation");

    assert_nested_city_error(&error, "address.city");

    assert_eq!(
        Customer::count_from(doc! {}, &client).await?,
        0,
        "the hook-invalidated document must not be written"
    );

    Ok(())
}

// Run test: cargo nextest run update_expressions_remain_non_validating
#[tokio::test]
async fn update_expressions_remain_non_validating() -> TestResult {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_update_boundary")]
    struct Customer {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        address: Address,
    }

    let client = client().await?;
    Customer::clear_from(&client).await?;

    let id = Customer::new()
        .address(Address::new().city("Springfield"))
        .save_from(&client)
        .await?;

    // Update expressions modify stored documents directly and intentionally
    // never run whole-model validation — nested validation does not change
    // this documented boundary.
    let result =
        Customer::update_by_id_from(id, doc! { "$set": { "address.city": "" } }, &client).await?;

    assert_eq!(result.modified_count, 1);

    let stored = Customer::find_by_id_from(id, &client)
        .await?
        .expect("the updated document should still exist");

    assert_eq!(
        stored.address.city, "",
        "the update path may durably store a nested-invalid value"
    );

    Ok(())
}
