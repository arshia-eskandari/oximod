# OxiMod Audit Remediation — D3 Authorization

## Status

D3 AUTHORIZED — SR-8 OPTION<T> OPERATOR REMEDIATION

## Governing documents

All existing remediation controls remain binding:

- REMEDIATION_BASELINE.md
- REMEDIATION_LEDGER.md
- REMEDIATION_MANDATE.md
- REMEDIATION_DIAGNOSIS.md
- REMEDIATION_D1_REVERIFY.md
- REMEDIATION_D2_AUTHORIZATION.md

The frozen black-box audit remains immutable.

## D3 scope

D3 may resolve exactly:

- SR-8

No other SR item and no P-item is authorized.

SR-8 maintainer decision:

  FIX_PRE_1_0

## Goal

Expose the already-supported ordered and numeric/modulo operator families on:

  Field<Option<T>>

using INNER-TYPE arguments.

The API must allow useful expressions such as conceptually:

  optional_i64.gt(10)
  optional_date.gte(date)
  optional_i32.inc(1)

It must NOT require or encourage:

  optional_i64.gt(Some(10))
  optional_i32.inc(Some(1))

and it must NOT accept:

  optional_i64.gt(None)
  optional_i32.inc(None)

The Option wrapper describes the stored field's nullability. Operator operands
remain values of the inner type.

## Approved design direction

Use dedicated inherent implementations for `Field<Option<T>>`, following the
existing bare-field implementations and their BSON expression semantics.

For ordered comparisons, preserve the same capability constraints as the
existing ordered operators on `Field<T>`, applied to the inner `T`.

The intended shape from D0 is equivalent in substance to:

  T: OrderedQueryValue + Into<Bson>
  operand: V where V: Into<T>

Do not achieve this by blanket-implementing marker traits for `Option<T>` if
doing so would make nonsensical operands such as `gt(None)` type-check.

For numeric/modulo operations, follow the same inner-value principle and the
existing bare-field constraints.

## Operator scope

D3 should expose exactly the existing PUBLIC ordered and numeric/modulo
families that are meaningful for the inner type and currently available on
the corresponding bare `Field<T>` surface.

This includes the audit-relevant families such as:

- ordered comparisons (`gt`, `gte`, `lt`, `lte`);
- numeric update operations such as `$inc` and any sibling numeric operation
  already belonging to the same existing bare-field numeric API;
- modulo query support where it belongs to the existing numeric family.

Do not invent new MongoDB operators.

Do not rename existing methods.

Do not alter the behavior of existing bare-field methods.

## Explicit exclusion: bitwise / integer-only expansion

Do NOT add the optional bitwise/integer-only family in D3.

Although D0 allowed it if it proved to be a trivial application of the same
design, it was not required by the measured workflows and is intentionally
excluded here to keep D3 bounded.

Do not add:

- new bitwise Option<T> operators;
- new bare-u64/ObjectId/enum operators;
- any P-7 work.

## Null / missing semantics

Do not add special client-side null handling.

The generated expressions should retain ordinary MongoDB semantics when the
stored field is null or missing.

Add a concise documentation sentence explaining that ordered comparisons on
an optional field compare against the supplied INNER value; null/missing
documents follow MongoDB's normal query semantics.

Do not change `is_null`, `is_not_null`, `exists`, `unset`, or other existing
Option-specific semantics.

## Implementation discipline

Prefer the smallest implementation that mirrors and reuses existing operator
construction logic.

Avoid copying BSON-expression construction if an existing internal helper can
be reused cleanly.

However, do not perform unrelated refactoring merely to deduplicate a few
lines.

No macro changes are expected. If the implementation unexpectedly appears to
require proc-macro changes, STOP and report before proceeding.

Do not modify model derivation.

Do not change persisted BSON formats.

Do not change validation.

Do not change indexes.

Do not change error variants.

Do not add dependencies.

Do not intentionally modify Cargo.lock.

Do not bump versions.

## Regression requirements

Add focused permanent regressions.

At minimum cover:

1. ordered Option<i64> filtering against INNER numeric values;
2. ordered Option<DateTime> filtering against INNER DateTime values;
3. numeric mutation of Option<i32> using an INNER operand, including `$inc`;
4. exact generated BSON/expression shape where suitable in oximod_core tests;
5. compatibility of existing bare-type ordered/numeric operators.

Add compile-surface protection proving that nonsensical Option operands do
not become valid.

At minimum ensure examples equivalent to:

  optional_i32.gt(None)
  optional_i32.inc(None)

remain compile failures.

Do not weaken the existing boolean ordered-comparison rejection.

Prefer existing test organization such as:

- `oximod/tests/field_queries.rs`
- directly relevant update integration tests
- focused oximod_core field/expression unit tests
- `oximod/tests/ui/query` where compile-fail coverage is appropriate

Do not mechanically import the complete black-box audit projects.

## Regression-first discipline

Where practical:

1. add the focused regression;
2. demonstrate that the relevant positive case fails against the pre-D3
   implementation;
3. implement the correction;
4. demonstrate that it passes afterward.

Do not alter an expected failure merely to make the implementation appear
green.

## Documentation

Update only documentation directly necessary to make the new Option<T>
operator surface discoverable and precise.

Document the inner-value operand convention.

Document the normal MongoDB null/missing semantics in one concise location.

Do not reopen or rewrite the completed D2 documentation pass except where a
small SR-8-specific addition is necessary.

## Explicitly out of scope

D3 must NOT implement or alter:

- SR-2 error-contract work;
- SR-3 index initialization;
- any CLOSED SR item;
- nested validation;
- index lifecycle;
- compound/partial indexes;
- pagination behavior;
- embedded BSON-conversion generation;
- bitwise/integer-only Option<T> expansion;
- P-1 through P-12.

If implementation appears to depend on another SR or P-item, STOP and report.

## Frozen audit

The frozen audit may be read to recover SR-8 evidence:

- W2-F01
- W2-V01
- W2-A7-B02
- W2-A3-B01
- relevant F-V01 coherence evidence

It must not be modified.

## Verification

Run focused tests during development.

Before declaring D3 complete, run at minimum:

- `cargo fmt --all -- --check`
- `cargo test -p oximod_core`
- `cargo test -p oximod_macros`
- `cargo test -p oximod --test compile_fail`
- directly relevant SR-8 integration tests
- `cargo test --doc --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Then run the MongoDB-backed suite:

  MONGODB_URI="mongodb://127.0.0.1:27019/?replicaSet=rs0" cargo nextest run

Use the true `cargo nextest` exit status.

If output is piped, enable `set -o pipefail` or preserve
`${PIPESTATUS[0]}`.

The previously observed TTL-monitor environmental condition must not be fixed,
skipped, weakened, or used to hide another failure. If the exact known TTL
failure reappears, report it separately with the surrounding suite totals.

Before completion also run:

- `git status --short`
- `git diff --name-only`
- `git diff --check`

Review the entire diff.

## Git discipline

Do not commit.

Do not push.

Do not rebase, merge, reset, amend, cherry-pick, or alter history.

Do not modify another worktree.

## D3 stopping condition

STOP when SR-8 has:

- focused regressions;
- implementation;
- directly relevant documentation;
- repository verification.

Do not mark SR-8 CLOSED.

Do not proceed to SR-3 or SR-2.

SR-8 remains subject to source-hidden external re-verification before
maintainer closure.

## Completion report

Report:

1. branch;
2. starting and ending HEAD;
3. every changed/added/deleted file;
4. exact operator families added to Field<Option<T>>;
5. exact type bounds and operand convention;
6. whether any helper/refactor was introduced and why;
7. regressions added;
8. evidence that inner operands compile and None operands do not;
9. exact BSON generated for representative ordered and numeric operations;
10. MongoDB integration behavior;
11. all commands and true exit statuses;
12. full nextest totals and any TTL-only environmental failure;
13. documentation changes;
14. compatibility impact;
15. confirmation that bitwise/integer-only work was excluded;
16. confirmation that SR-2/SR-3 and all P-items were untouched;
17. confirmation that the frozen audit was untouched;
18. final git status, name-only diff, and diff-check.

Then STOP and wait for maintainer review.
