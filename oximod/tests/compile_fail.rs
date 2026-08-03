//! Compile-fail coverage for the typed query API.
//!
//! The trybuild cases verify that query and update operators are available
//! only for compatible field and element types.

#[test]
fn typed_queries_reject_invalid_operations() {
    let tests = trybuild::TestCases::new();

    tests.compile_fail("tests/ui/query/*.rs");
}
