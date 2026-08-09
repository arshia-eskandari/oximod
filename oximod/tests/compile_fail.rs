//! Compile-fail coverage for the typed query API and derive diagnostics.
//!
//! The trybuild cases verify that query and update operators are available
//! only for compatible field and element types, that conflicting `#[index]`
//! declarations are rejected at compile time, and that legitimate multi-index
//! models continue to compile.

#[test]
fn typed_queries_reject_invalid_operations() {
    let tests = trybuild::TestCases::new();

    tests.compile_fail("tests/ui/query/*.rs");
}

#[test]
fn conflicting_index_declarations_are_rejected() {
    let tests = trybuild::TestCases::new();

    tests.compile_fail("tests/ui/index/two_text_indexes.rs");
    tests.compile_fail("tests/ui/index/duplicate_index_name.rs");
    tests.pass("tests/ui/index/multiple_distinct_indexes.rs");
}

#[test]
fn standard_struct_attributes_are_accepted_and_unknown_attributes_are_named() {
    let tests = trybuild::TestCases::new();

    tests.pass("tests/ui/attrs/standard_attributes.rs");
    tests.compile_fail("tests/ui/attrs/unknown_attribute.rs");
}
