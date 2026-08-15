//! Integration tests for `#[validate(nested)]` through the generated
//! inherent `validate()` method.
//!
//! These tests cover the pure validation semantics: explicit opt-in descent,
//! supported container shapes, path-aware error attribution, deterministic
//! ordering, and aggregation across parent and descendant errors. They never
//! touch MongoDB; persistence-path coverage lives in the nested save, bulk,
//! and session test files.

use std::collections::HashMap;

use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiModError, ValidationError};
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Debug)]
#[model(embedded)]
struct Address {
    #[validate(non_empty)]
    city: String,

    #[validate(pattern = "^[0-9]{5}$")]
    postal_code: String,
}

fn valid_address() -> Address {
    Address::new().city("Springfield").postal_code("12345")
}

fn invalid_address() -> Address {
    Address::new().city("").postal_code("12345")
}

fn error_fields(error: &OxiModError) -> Vec<&str> {
    error
        .validation_errors()
        .expect("a Validation error should be returned")
        .iter()
        .map(|validation_error| validation_error.field.as_str())
        .collect()
}

fn single_error(error: &OxiModError) -> &ValidationError {
    let errors = error
        .validation_errors()
        .expect("a Validation error should be returned");

    assert_eq!(errors.len(), 1, "expected exactly one error: {errors:?}");
    &errors[0]
}

// Run test: cargo nextest run direct_nested_child_errors_are_path_prefixed
#[test]
fn direct_nested_child_errors_are_path_prefixed() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_direct_child")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        address: Address,
    }

    let error = User::new()
        .address(invalid_address())
        .validate()
        .expect_err("an invalid nested child should fail parent validation");

    let validation_error = single_error(&error);

    assert_eq!(validation_error.field, "address.city");
    assert_eq!(validation_error.message, "must be non-empty");
}

// Run test: cargo nextest run valid_nested_child_passes_parent_validation
#[test]
fn valid_nested_child_passes_parent_validation() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_valid_child")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        address: Address,
    }

    let result = User::new().address(valid_address()).validate();

    assert!(matches!(result, Ok(())));
}

// Run test: cargo nextest run fields_without_nested_keep_no_descent_behavior
#[test]
fn fields_without_nested_keep_no_descent_behavior() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_opt_out")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        address: Address,
    }

    let user = User::new().address(invalid_address());

    assert!(
        matches!(user.validate(), Ok(())),
        "without #[validate(nested)] the parent must not descend"
    );

    let error = user
        .address
        .validate()
        .expect_err("the embedded value's own validate() still rejects");

    assert_eq!(single_error(&error).field, "city");
}

// Run test: cargo nextest run optional_nested_child_is_skipped_when_none
#[test]
fn optional_nested_child_is_skipped_when_none() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_option")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        address: Option<Address>,
    }

    let absent = User::new();
    assert!(matches!(absent.validate(), Ok(())));

    let error = User::new()
        .address(invalid_address())
        .validate()
        .expect_err("an invalid Some value should fail");

    assert_eq!(single_error(&error).field, "address.city");
}

// Run test: cargo nextest run required_and_nested_compose_on_an_optional_child
#[test]
fn required_and_nested_compose_on_an_optional_child() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_required_option")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(required, nested)]
        address: Option<Address>,
    }

    let error = User::new()
        .validate()
        .expect_err("None should fail because of `required`");

    let validation_error = single_error(&error);

    assert_eq!(validation_error.field, "address");
    assert_eq!(validation_error.message, "is required");

    let error = User::new()
        .address(invalid_address())
        .validate()
        .expect_err("an invalid Some value should fail through descent");

    assert_eq!(single_error(&error).field, "address.city");
}

// Run test: cargo nextest run vector_elements_are_validated_in_ascending_index_order
#[test]
fn vector_elements_are_validated_in_ascending_index_order() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_vec")]
    struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        addresses: Vec<Address>,
    }

    let error = Order::new()
        .addresses(vec![
            invalid_address(),
            valid_address(),
            Address::new().city("Shelbyville").postal_code("bad"),
        ])
        .validate()
        .expect_err("invalid vector elements should fail");

    assert_eq!(
        error_fields(&error),
        ["addresses[0].city", "addresses[2].postal_code"]
    );
}

// Run test: cargo nextest run empty_vector_composes_with_non_empty_and_nested
#[test]
fn empty_vector_composes_with_non_empty_and_nested() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_vec_non_empty")]
    struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        nested_only: Vec<Address>,

        #[validate(non_empty, nested)]
        must_have_items: Vec<Address>,
    }

    let error = Order::new()
        .must_have_items(Vec::<Address>::new())
        .validate()
        .expect_err("the empty non_empty vector should fail");

    let validation_error = single_error(&error);

    assert_eq!(validation_error.field, "must_have_items");
    assert_eq!(validation_error.message, "must be non-empty");

    let result = Order::new()
        .must_have_items(vec![valid_address()])
        .validate();

    assert!(
        matches!(result, Ok(())),
        "an empty vector is not invalid merely because it is nested"
    );
}

// Run test: cargo nextest run recursive_container_compositions_reach_the_embedded_leaf
#[test]
fn recursive_container_compositions_reach_the_embedded_leaf() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_recursive_containers")]
    struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        maybe_each: Vec<Option<Address>>,

        #[validate(nested)]
        maybe_all: Option<Vec<Address>>,

        #[validate(nested)]
        keyed_batches: HashMap<String, Vec<Option<Address>>>,
    }

    let order = Order::new()
        .maybe_each(vec![None, Some(invalid_address()), None])
        .maybe_all(vec![valid_address(), invalid_address()])
        .keyed_batches(HashMap::from([(
            "west".to_owned(),
            vec![Some(invalid_address())],
        )]));

    let error = order
        .validate()
        .expect_err("invalid leaves through recursive containers should fail");

    assert_eq!(
        error_fields(&error),
        [
            "maybe_each[1].city",
            "maybe_all[1].city",
            "keyed_batches[\"west\"][0].city",
        ]
    );

    let empty = Order::new();
    assert!(
        matches!(empty.validate(), Ok(())),
        "empty containers and None produce no nested errors"
    );
}

// Run test: cargo nextest run map_entries_report_in_sorted_key_order_with_escaped_keys
#[test]
fn map_entries_report_in_sorted_key_order_with_escaped_keys() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_map")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        addresses: HashMap<String, Address>,
    }

    let error = User::new()
        .addresses(HashMap::from([
            ("shipping".to_owned(), invalid_address()),
            ("billing".to_owned(), invalid_address()),
            ("archived".to_owned(), valid_address()),
        ]))
        .validate()
        .expect_err("invalid map entries should fail");

    assert_eq!(
        error_fields(&error),
        [
            "addresses[\"billing\"].city",
            "addresses[\"shipping\"].city"
        ]
    );

    let error = User::new()
        .addresses(HashMap::from([(
            "home.old [main] \"x\"".to_owned(),
            invalid_address(),
        )]))
        .validate()
        .expect_err("a punctuated map key should still fail");

    assert_eq!(
        single_error(&error).field,
        "addresses[\"home.old [main] \\\"x\\\"\"].city"
    );
}

// Run test: cargo nextest run deep_nesting_composes_paths_across_every_opted_in_edge
#[test]
fn deep_nesting_composes_paths_across_every_opted_in_edge() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[model(embedded)]
    struct Shipping {
        #[validate(nested)]
        address: Address,

        #[validate(nested)]
        stops: Vec<Address>,
    }

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[model(embedded)]
    struct Fulfillment {
        #[validate(nested)]
        shipping: Shipping,
    }

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_deep")]
    struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        fulfillment: Fulfillment,
    }

    let order = Order::new().fulfillment(
        Fulfillment::new().shipping(
            Shipping::new()
                .address(Address::new().city("Springfield").postal_code("bad"))
                .stops(vec![valid_address(), invalid_address()]),
        ),
    );

    let error = order
        .validate()
        .expect_err("a three-level nested violation should fail");

    assert_eq!(
        error_fields(&error),
        [
            "fulfillment.shipping.address.postal_code",
            "fulfillment.shipping.stops[1].city",
        ]
    );
}

// Run test: cargo nextest run nested_descent_stops_at_edges_without_the_attribute
#[test]
fn nested_descent_stops_at_edges_without_the_attribute() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[model(embedded)]
    struct Shipping {
        // No `nested` here: this containment edge does not opt in.
        address: Address,
    }

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_edge_opt_in")]
    struct Order {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        shipping: Shipping,
    }

    let order = Order::new().shipping(Shipping::new().address(invalid_address()));

    assert!(
        matches!(order.validate(), Ok(())),
        "descent must stop at containment edges that did not opt in"
    );
}

// Run test: cargo nextest run parent_and_descendant_errors_aggregate_in_declaration_order
#[test]
fn parent_and_descendant_errors_aggregate_in_declaration_order() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_aggregation")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(non_empty)]
        name: String,

        #[validate(nested)]
        address: Address,

        #[validate(nested)]
        previous_addresses: Vec<Address>,
    }

    let user = User::new()
        .name("")
        .address(Address::new().city("").postal_code("bad"))
        .previous_addresses(vec![invalid_address(), invalid_address()]);

    let error = user
        .validate()
        .expect_err("parent and descendant violations should all fail");

    assert_eq!(
        error_fields(&error),
        [
            "name",
            "address.city",
            "address.postal_code",
            "previous_addresses[0].city",
            "previous_addresses[1].city",
        ],
        "errors must aggregate depth-first in declaration order"
    );
}

// Run test: cargo nextest run nested_paths_use_rust_field_names_not_serde_names
#[test]
fn nested_paths_use_rust_field_names_not_serde_names() {
    #[derive(Model, Serialize, Deserialize, Debug)]
    #[model(embedded)]
    struct Renamed {
        #[serde(rename = "postalCode")]
        #[validate(non_empty)]
        postal_code: String,
    }

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("nested_serde_rename")]
    #[serde(rename_all = "camelCase")]
    struct User {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[validate(nested)]
        home_address: Renamed,
    }

    let error = User::new()
        .home_address(Renamed::new().postal_code(""))
        .validate()
        .expect_err("the renamed nested field should fail");

    assert_eq!(
        single_error(&error).field,
        "home_address.postal_code",
        "validation attribution keeps Rust identifiers, not Serde/BSON names"
    );
}
