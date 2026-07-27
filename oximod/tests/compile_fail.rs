#[test]
fn typed_queries_reject_invalid_operations() {
    let tests = trybuild::TestCases::new();

    tests.compile_fail("tests/ui/query/*.rs");
}
