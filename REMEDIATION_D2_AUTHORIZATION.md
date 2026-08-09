# OxiMod Audit Remediation — D2 Authorization

## Status

D2 AUTHORIZED — DOCUMENTATION TRUTH PASS

## Governing documents

This file authorizes D2.

All existing remediation controls remain binding:

- REMEDIATION_BASELINE.md
- REMEDIATION_LEDGER.md
- REMEDIATION_MANDATE.md
- REMEDIATION_DIAGNOSIS.md
- REMEDIATION_D1_REVERIFY.md

The frozen black-box audit remains immutable.

## D2 scope

D2 may resolve exactly:

- SR-1
- SR-4
- SR-5
- SR-9
- SR-10
- SR-11
- SR-13

These are documentation-only resolutions.

No code/API implementation for these items is authorized except the minimum
mechanical changes necessary to make documentation examples compile where
explicitly required by SR-11.

Do not implement any other SR item.

Do not implement any P-1 through P-12 item.

## SR-1 — embedded validation boundary

Document clearly that validation does NOT automatically recurse into embedded
models.

Document both supported remedies:

1. a custom validator on the containing field;
2. hooks used as a save guard.

If hooks are presented as a remedy, explicitly state that users must cover
both `pre_save` and `pre_save_mut` when both save forms are used.

Do NOT implement:

- automatic recursive validation;
- `#[validate(nested)]`;
- new validation path semantics.

Place the warning where users naturally encounter validation, including the
crate-level validation documentation, README validation guidance, and derive /
field-attribute documentation where practical.

## SR-4 — derived composite-key warning

Document that a derived or mirrored composite-key field protected by
`#[index(unique)]` is NOT a safe substitute for a real MongoDB compound unique
index.

Explain that partial updates can change source fields without recomputing a
derived mirror field.

Route compound uniqueness to the raw MongoDB collection/index API.

Do not add compound-index support.

## SR-5 — partial / filtered index boundary

Document that partial / filtered indexes
(`partialFilterExpression`) are a raw-driver boundary.

Explain that they should be established through the underlying MongoDB
collection.

Do not add `partial_filter` or similar `#[index]` syntax.

Do not imply that OxiMod validation replaces MongoDB unique-index enforcement.

## SR-9 — Vec<Embedded> array/operator ergonomics

Correct the audit-era discoverability problem.

Document that typed embedded-array matching already exists through
`elem_match_nested`.

Show the supported spelling clearly enough that a consumer can discover it.

For mutation operations requiring conversion of embedded values to BSON,
document the consumer-side conversion/workaround accurately.

Do NOT:

- reimplement `elem_match_nested`;
- generate `From<Embedded> for Bson`;
- alter array operator semantics.

Prefer compile-checked examples where practical.

## SR-10 — production rules

Publish the five measured production rules identified in D0:

1. transactions/sessions remain a raw-driver escape hatch;
2. schema evolution should update dotted fields rather than replacing whole
   typed models when old documents may differ;
3. `serde(alias)` is a read-compatibility tool, not a persisted-field rename
   migration;
4. avoid relying on bare `serde(default)` as a complete schema-evolution
   strategy for persisted documents;
5. global-client initialization is process-level / one-time and callers should
   handle its result accordingly.

Keep wording precise and operational.

Do not turn any of these boundaries into new APIs.

## SR-11 — broken published examples

Fix the published documentation examples identified by the audit that refer to
undeclared fields or otherwise fail as written.

Convert documentation examples to compiled doctests where practical.

Prefer `no_run` when the example requires MongoDB/runtime setup but should
still type-check.

Do not make examples materially harder to read merely to satisfy doctest
mechanics.

Do not weaken examples by changing them to `ignore` unless there is a specific,
documented reason that compilation itself cannot be checked.

## SR-13 — page()/all() poison-document behavior

Document the actual measured behavior:

- `.all()` fails when deserialization encounters a poisoned document;
- `.page()` uses the same terminal behavior for its selected window;
- a poisoned page window therefore fails rather than silently dropping bad
  documents;
- later pages not containing the poisoned documents may succeed because they
  operate on a different window.

Document the raw-document escape/repair route where appropriate.

Do NOT:

- alter `.page()` behavior;
- add poison-tolerant enumeration;
- add `_id` error enrichment in D2.

The optional offending-document `_id` context belongs with the later SR-2
error-surface phase.

## Explicitly out of scope

D2 must NOT implement or materially alter behavior for:

- SR-2
- SR-3
- SR-6
- SR-7
- SR-8
- SR-12
- any P-item

SR-6, SR-7, and SR-12 are already CLOSED and must not be reopened or modified
without explicit maintainer approval.

Do not:

- redesign OxiModError;
- add index-initialization APIs;
- add Option<T> query operators;
- add nested validation;
- add new index features;
- change query/pagination runtime semantics;
- perform unrelated refactors.

## Documentation discipline

Every new statement must be supported by:

- frozen audit evidence,
- D0 source-aware diagnosis,
- existing public behavior,
- or existing API surface.

Do not invent capabilities.

Do not soften important limitations into vague wording.

At the same time, do not present raw-driver escape hatches as defects where
they are intentional boundaries.

Avoid duplicating long explanations unnecessarily; establish one authoritative
explanation and link/cross-reference where appropriate.

README and rustdoc should not contradict each other.

## Change discipline

Make the smallest coherent documentation changes.

Do not add dependencies.

Do not bump versions.

Do not modify Cargo.lock intentionally.

Do not commit.

Do not push.

Do not modify another worktree.

Do not modify the frozen audit.

If resolving one item appears to require a behavior/API change, STOP that item
and report it rather than expanding scope.

## Verification

Run focused documentation checks while editing.

Before declaring D2 complete, run at minimum:

- `cargo fmt --all -- --check`
- `cargo test --doc --workspace`
- `cargo test -p oximod_core -p oximod_macros`
- `cargo test -p oximod --test compile_fail`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Also inspect all changed README/rustdoc examples for consistency.

If documentation changes require a directly relevant integration test to
confirm a statement, run the smallest such test.

Do NOT run a new black-box audit.

The MongoDB TTL environment issue is irrelevant unless a directly invoked
existing test happens to encounter it. Do not modify or weaken that test.

Before completion run:

- `git status --short`
- `git diff --name-only`
- `git diff --check`

## D2 stopping condition

STOP when SR-1, SR-4, SR-5, SR-9, SR-10, SR-11, and SR-13 have received their
authorized documentation resolutions and documentation/example verification
has completed.

Do not proceed to D3.

Do not commit.

## Completion report

Report:

1. branch;
2. starting and ending HEAD;
3. every file changed;
4. exact documentation resolution for each SR-1, SR-4, SR-5, SR-9, SR-10,
   SR-11, SR-13;
5. examples converted to compiled doctests and why `no_run`/other form was
   chosen;
6. commands run and results;
7. any statement that could not be documented confidently;
8. any blocked item;
9. any compatibility impact;
10. confirmation that no behavior/API implementation occurred;
11. confirmation that no other SR or P-item was touched;
12. confirmation that the frozen audit was untouched;
13. final git status / name-only diff / diff-check results.

Then wait for maintainer review.
