# OxiMod Audit Remediation — D1 Authorization

## Status

D1 AUTHORIZED — BOUNDED IMPLEMENTATION

## Governing documents

This file authorizes D1 and supersedes only the D0-specific
"diagnosis-only / no implementation" restrictions in REMEDIATION_MANDATE.md.

All other rules in these files remain binding:

- REMEDIATION_BASELINE.md
- REMEDIATION_LEDGER.md
- REMEDIATION_MANDATE.md
- REMEDIATION_DIAGNOSIS.md

The frozen audit remains immutable.

## D1 scope

D1 may implement exactly:

- SR-6
- SR-7
- SR-12

No other SR item is authorized for implementation in D1.

No P-1 through P-12 item is authorized.

## SR-6 authorization

Implement `exists()` as a true existence probe that does not deserialize the
matched model.

Approved behavior:

A matching document that cannot deserialize as the model may still yield
`Ok(true)` from `exists()`.

Requirements:

- preserve the existing public signature;
- do not alter `find_by_id`, `first`, `all`, or other typed-read semantics;
- add a regression using a raw-inserted malformed matching document;
- assert agreement with `count(filter) > 0`;
- update directly affected rustdoc if it currently describes the old
  typed-deserialization implementation.

## SR-7 authorization

Add an earlier compile-time signal for exactly these declaration-local
conflicts:

1. more than one text-implying index declaration on one derived model;
2. duplicate literal `#[index(name = "...")]` values on one derived model.

Requirements:

- use the same text-implying predicate as index generation;
- do not attempt broad MongoDB index compatibility validation;
- do not attempt cross-model / shared-collection global analysis;
- preserve all valid existing index declarations;
- add compile-fail/UI regressions for both approved conflicts;
- retain positive coverage proving ordinary multi-index models still compile;
- enrich the runtime index-creation error with collection context without
  changing its error variant.

## SR-12 authorization

Implement:

1. acceptance of ordinary/inert standard Rust struct attributes through an
   explicit skip-list;
2. remaining unsupported-attribute diagnostics that name the offending
   attribute.

Do NOT ignore all unknown attributes.

The existing rejection of genuinely unknown/unregistered attributes must
remain covered.

Approved standard-attribute handling should include the D0-measured cases
(`doc`, `allow`, `non_exhaustive`) and may include the other ordinary standard
attributes identified in D0 where doing so is safe and mechanically
consistent.

### `_id` diagnostic

A targeted `_id` diagnostic is conditionally authorized.

Implement it only if it can improve diagnostics without rejecting source that
0.3.0 already accepts through type aliases, qualified paths, or equivalent
valid type spellings.

If a reliable macro-level check cannot be made without narrowing accepted
source, do NOT implement that sub-part. Record why and leave the existing
behavior unchanged for later design review.

Add regression coverage for every diagnostic behavior changed.

## Explicitly out of scope

D1 must NOT implement or materially edit behavior for:

- SR-1
- SR-2
- SR-3
- SR-4
- SR-5
- SR-8
- SR-9
- SR-10
- SR-11
- SR-13
- any P-item

In particular, do not:

- add nested validation;
- redesign OxiModError;
- add init_indexes;
- add Option<T> operator APIs;
- add embedded BSON conversion impls;
- add partial-index support;
- change pagination/deserialization behavior;
- perform unrelated refactors.

If an authorized change reveals that another SR must change first, STOP and
report the dependency rather than expanding scope.

## Change discipline

Preserve every existing test unless an authorized behavior change makes a
specific expectation obsolete.

Do not weaken tests.

Prefer regression-first implementation where practical.

Make the smallest coherent change.

Do not add dependencies.

Do not bump versions.

Do not commit.

Do not push.

Do not modify another worktree.

Do not modify the frozen audit.

## Verification

Run focused tests during implementation.

Before declaring D1 complete, run at minimum:

- `cargo fmt --all -- --check`
- `cargo test -p oximod_core -p oximod_macros`
- `cargo test -p oximod --test compile_fail`
- the directly relevant OxiMod integration tests
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

With MongoDB available, also run:

- `MONGODB_URI="mongodb://127.0.0.1:27019/?replicaSet=rs0" cargo nextest run`

The previously observed TTL failure must NOT be "fixed", skipped, weakened, or
attributed to D1 merely because it reappears. If it reappears, report its
server-state evidence separately.

At completion also run:

- `git status --short`
- `git diff --name-only`
- `git diff --check`

## D1 stopping condition

STOP when SR-6, SR-7, and SR-12 are implemented and tested, or when an
authorized item encounters a maintainer decision that cannot safely be made
inside this authorization.

Do not proceed to D2.

Do not commit.

Report:

- starting and ending HEAD;
- files changed;
- exact behavior changed for SR-6, SR-7, SR-12;
- tests added/changed;
- all commands run and results;
- whether the optional `_id` diagnostic shipped or was deferred, and why;
- any compatibility impact discovered beyond D0;
- any failed or blocked work;
- confirmation that no other SR/P item was implemented;
- confirmation that the frozen audit was untouched.
