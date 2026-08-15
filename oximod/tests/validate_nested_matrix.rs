//! Audit-closure matrix for `#[validate(nested)]`.
//!
//! The 0.3.0 black-box audit measured a 135-cell matrix — 9 container
//! shapes × 5 validation-rule kinds × 3 validated call forms — in which
//! every parent-level validation returned `Ok` and violating documents were
//! durably written, because validation did not descend into embedded
//! models. This suite recreates that matrix with `#[validate(nested)]` on
//! the containment edges and asserts the opposite outcome: every
//! intentionally invalid cell now rejects with a path-aware validation
//! error, and no invalid document is ever written.
//!
//! Positive controls confirm that fully valid values still validate and
//! save, and that the parent's own top-level rules keep rejecting.

mod common;

use std::collections::HashMap;

use common::init;
use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiModError};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

mod validators {
    pub fn reject_zebra(value: &str) -> Result<(), String> {
        if value == "zebra" {
            return Err("value 'zebra' is not allowed".into());
        }

        Ok(())
    }
}

/// One field per historical rule family: `non_empty`, `max_length`,
/// numeric `min`/`max`, `pattern`, and `custom(fn)`.
#[derive(Model, Serialize, Deserialize, Debug, Clone)]
#[model(embedded)]
struct RuleLeaf {
    #[validate(non_empty)]
    text: String,

    #[validate(max_length = 5)]
    capped: String,

    #[validate(min = 1, max = 10)]
    level: i32,

    #[validate(pattern = "^[0-9]{5}$")]
    code: String,

    #[validate(custom(crate::validators::reject_zebra))]
    tag: String,
}

/// Two-level embedding: the audit probed both a direct `.leaf` and a
/// `.leaves[i]` chain through an intermediate embedded model.
#[derive(Model, Serialize, Deserialize, Debug, Clone)]
#[model(embedded)]
struct Middle {
    #[validate(nested)]
    leaf: RuleLeaf,

    #[validate(nested)]
    leaves: Vec<RuleLeaf>,
}

/// One field per historical container shape, every containment edge
/// explicitly opted in with `#[validate(nested)]`.
#[derive(Model, Serialize, Deserialize, Debug)]
#[db("test")]
#[collection("validate_nested_matrix")]
struct MatrixDoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(non_empty)]
    title: String,

    #[validate(nested)]
    direct: RuleLeaf,

    #[validate(nested)]
    optional: Option<RuleLeaf>,

    #[validate(nested)]
    one: Vec<RuleLeaf>,

    #[validate(nested)]
    many: Vec<RuleLeaf>,

    #[validate(nested)]
    maybe_each: Vec<Option<RuleLeaf>>,

    #[validate(nested)]
    keyed: HashMap<String, RuleLeaf>,

    #[validate(nested)]
    middle: Middle,
}

const RULES: [&str; 5] = ["non_empty", "max_length", "min_max", "pattern", "custom"];

const SHAPES: [&str; 9] = [
    "direct",
    "option_some",
    "vec_len1_idx0",
    "vec_len3_idx0",
    "vec_len3_idx2",
    "vec_option",
    "hashmap",
    "two_level_direct",
    "two_level_vec",
];

fn valid_leaf() -> RuleLeaf {
    RuleLeaf::new()
        .text("ok")
        .capped("abc")
        .level(5)
        .code("12345")
        .tag("fine")
}

/// Returns a leaf violating exactly one rule family, and the leaf field the
/// resulting validation error must be attributed to.
fn invalid_leaf(rule: &str) -> (RuleLeaf, &'static str) {
    let leaf = valid_leaf();

    match rule {
        "non_empty" => (leaf.text(""), "text"),
        "max_length" => (leaf.capped("toolong"), "capped"),
        "min_max" => (leaf.level(0), "level"),
        "pattern" => (leaf.code("bad"), "code"),
        "custom" => (leaf.tag("zebra"), "tag"),
        _ => unreachable!("unknown rule family: {rule}"),
    }
}

fn valid_doc() -> MatrixDoc {
    MatrixDoc::new()
        .title("valid")
        .direct(valid_leaf())
        .optional(valid_leaf())
        .one(vec![valid_leaf()])
        .many(vec![valid_leaf(), valid_leaf(), valid_leaf()])
        .maybe_each(vec![None, Some(valid_leaf())])
        .keyed(HashMap::from([("west".to_owned(), valid_leaf())]))
        .middle(
            Middle::new()
                .leaf(valid_leaf())
                .leaves(vec![valid_leaf(), valid_leaf()]),
        )
}

/// Builds a document that is valid everywhere except one leaf placed at the
/// given container shape, violating the given rule family. Returns the
/// document and the exact validation-error path expected for the cell.
fn invalid_doc(shape: &str, rule: &str) -> (MatrixDoc, String) {
    let (bad, field) = invalid_leaf(rule);
    let mut document = valid_doc();

    let path = match shape {
        "direct" => {
            document.direct = bad;
            format!("direct.{field}")
        }
        "option_some" => {
            document.optional = Some(bad);
            format!("optional.{field}")
        }
        "vec_len1_idx0" => {
            document.one = vec![bad];
            format!("one[0].{field}")
        }
        "vec_len3_idx0" => {
            document.many = vec![bad, valid_leaf(), valid_leaf()];
            format!("many[0].{field}")
        }
        "vec_len3_idx2" => {
            document.many = vec![valid_leaf(), valid_leaf(), bad];
            format!("many[2].{field}")
        }
        "vec_option" => {
            document.maybe_each = vec![None, Some(bad)];
            format!("maybe_each[1].{field}")
        }
        "hashmap" => {
            document.keyed = HashMap::from([("west".to_owned(), bad)]);
            format!("keyed[\"west\"].{field}")
        }
        "two_level_direct" => {
            document.middle.leaf = bad;
            format!("middle.leaf.{field}")
        }
        "two_level_vec" => {
            document.middle.leaves = vec![valid_leaf(), bad];
            format!("middle.leaves[1].{field}")
        }
        _ => unreachable!("unknown container shape: {shape}"),
    };

    (document, path)
}

fn assert_single_validation_error(error: &OxiModError, expected_path: &str, cell: &str) {
    let errors = error
        .validation_errors()
        .unwrap_or_else(|| panic!("cell {cell}: expected a Validation error, got {error:?}"));

    assert_eq!(
        errors.len(),
        1,
        "cell {cell}: expected exactly one error, got {errors:?}"
    );

    assert_eq!(
        errors[0].field, expected_path,
        "cell {cell}: wrong error path"
    );
}

// Run test: cargo nextest run all_invalid_matrix_cells_reject_through_parent_validate
#[tokio::test]
async fn all_invalid_matrix_cells_reject_through_parent_validate() -> TestResult {
    for shape in SHAPES {
        for rule in RULES {
            let cell = format!("validate/{shape}/{rule}");
            let (document, expected_path) = invalid_doc(shape, rule);

            let error = document
                .validate()
                .expect_err(&format!("cell {cell}: parent validate() must reject"));

            assert_single_validation_error(&error, &expected_path, &cell);
        }
    }

    assert!(
        matches!(valid_doc().validate(), Ok(())),
        "the fully valid control document must keep validating"
    );

    let error = valid_doc()
        .title("")
        .validate()
        .expect_err("the parent's own top-level rule must keep rejecting");

    assert_single_validation_error(&error, "title", "validate/top_level_control");

    Ok(())
}

/// Removes this call form's marker documents. The collection is shared by
/// the concurrently running per-call-form tests, so cleanup and counting are
/// scoped by `title` markers instead of clearing the whole collection.
async fn remove_marked(markers: [&str; 2]) -> TestResult {
    MatrixDoc::get_document_collection()?
        .delete_many(doc! { "title": { "$in": markers.to_vec() } })
        .await?;

    Ok(())
}

// Run test: cargo nextest run all_invalid_matrix_cells_reject_through_save
#[tokio::test]
async fn all_invalid_matrix_cells_reject_through_save() -> TestResult {
    init().await?;
    remove_marked(["save-invalid", "save-control"]).await?;

    for shape in SHAPES {
        for rule in RULES {
            let cell = format!("save/{shape}/{rule}");
            let (document, expected_path) = invalid_doc(shape, rule);

            let error = document
                .title("save-invalid")
                .save()
                .await
                .map(|_| ())
                .expect_err(&format!("cell {cell}: save() must reject"));

            assert_single_validation_error(&error, &expected_path, &cell);
        }
    }

    assert_eq!(
        MatrixDoc::count(doc! { "title": "save-invalid" }).await?,
        0,
        "no invalid document may be durably written through save()"
    );

    valid_doc().title("save-control").save().await?;

    assert_eq!(
        MatrixDoc::count(doc! { "title": "save-control" }).await?,
        1,
        "the valid control document must still save"
    );

    remove_marked(["save-invalid", "save-control"]).await?;
    Ok(())
}

// Run test: cargo nextest run all_invalid_matrix_cells_reject_through_save_mut
#[tokio::test]
async fn all_invalid_matrix_cells_reject_through_save_mut() -> TestResult {
    init().await?;
    remove_marked(["save_mut-invalid", "save_mut-control"]).await?;

    for shape in SHAPES {
        for rule in RULES {
            let cell = format!("save_mut/{shape}/{rule}");
            let (document, expected_path) = invalid_doc(shape, rule);
            let mut document = document.title("save_mut-invalid");

            let error = document
                .save_mut()
                .await
                .map(|_| ())
                .expect_err(&format!("cell {cell}: save_mut() must reject"));

            assert_single_validation_error(&error, &expected_path, &cell);
        }
    }

    assert_eq!(
        MatrixDoc::count(doc! { "title": "save_mut-invalid" }).await?,
        0,
        "no invalid document may be durably written through save_mut()"
    );

    valid_doc().title("save_mut-control").save_mut().await?;

    assert_eq!(
        MatrixDoc::count(doc! { "title": "save_mut-control" }).await?,
        1,
        "the valid control document must still save through save_mut()"
    );

    remove_marked(["save_mut-invalid", "save_mut-control"]).await?;
    Ok(())
}
