# OxiMod Audit Remediation — D6 Error-Contract Implementation Authorization

## Status

D6 IMPLEMENTATION AUTHORIZED — SR-2 ONLY

## Governing design

D6 implements SR-2 according to the completed D5 error-contract proposal,
subject to the maintainer amendments in this authorization.

SR-2 remains:

  FIX_AND_DOCUMENT_PRE_1_0

The goal is one coherent failure-class contract across OxiMod's public error
surface while preserving the original underlying errors through source().

## Approved variant model

Preserve exactly the existing ten public OxiModError variants.

Do NOT:

- add a variant;
- remove a variant;
- rename a variant;
- change public variant fields;
- make the enum non-exhaustive.

Approved semantic classes:

- Connection:
  MongoDB client/connectivity infrastructure failure, including connection
  establishment, authentication, DNS resolution, TLS configuration,
  server selection, transport I/O, and connection-pool failure.

  IMPORTANT:
  Connection does NOT guarantee that the logical operation was never sent,
  never reached MongoDB, or is safe to retry. No delivery or retry guarantee
  may be documented or inferred from this variant.

- GlobalClientInit:
  global-client initialization lifecycle failure.

- GlobalClientMissing:
  required global client has not been initialized.

- Serialization:
  Rust/BSON encoding or decoding failure, independent of call site.

- Aggregation:
  non-connectivity, non-serialization aggregation-domain failure.
  No new aggregation API is authorized.

- Index:
  non-connectivity, non-serialization index-domain failure.

- Validation:
  OxiMod model-validation failure.

- Database:
  general MongoDB/driver operation failure that is not classified as
  Connection, Serialization, Index, or Aggregation.

  Database is the conservative fallback. It does NOT guarantee that the
  server definitely received or rejected the operation.

- Custom:
  user-defined/hook/domain error propagated according to the existing
  contract.

- Query:
  typed-query configuration failure.

## Approved precedence

For operation-time mongodb::error::Error values:

  1. Connection
  2. Serialization
  3. operation domain: Index or Aggregation
  4. Database

Thus:

- connectivity failure during ordinary CRUD -> Connection;
- connectivity failure during index establishment -> Connection;
- connectivity failure during aggregation -> Connection;
- BSON serialization/deserialization -> Serialization;
- non-connectivity index rejection -> Index;
- non-connectivity aggregation rejection -> Aggregation;
- duplicate key / ordinary write rejection -> Database;
- remaining current/future driver kinds -> Database unless a higher rule
  applies.

## Approved mongodb 3.8.0 classification

Connection includes the currently enabled driver kinds:

- Io
- ServerSelection
- ConnectionPoolCleared
- DnsResolve
- InvalidTlsConfig
- Authentication

Serialization includes:

- BsonSerialization
- BsonDeserialization

All remaining enabled kinds and the #[non_exhaustive] fallback classify as
Database unless the Index/Aggregation operation domain applies.

Feature-gated kinds that are not enabled by the current workspace do not need
speculative implementation beyond a conservative fallback.

Do not classify by strings or Display text.

## Centralized classifier

Implement one centralized classification policy.

Generated and non-generated OxiMod call sites must not independently match
mongodb::error::ErrorKind.

Prefer a hidden internal-support namespace exposed only as necessary for
derive-macro expansion, following the repository's existing hidden re-export
conventions.

For example, a shape equivalent to:

  ::oximod::_error::classify_driver_error(...)

is preferred over presenting a normal-looking public user API.

If the repository architecture makes an inherent
OxiModError::from_driver(...) materially cleaner, it may be used only as
#[doc(hidden)] macro infrastructure.

In either design:

- do not recommend the hidden classifier to consumers;
- do not document it as normal public API;
- preserve the original mongodb::error::Error by value as source();
- carry operation context separately from classification;
- use an explicit operation-domain value rather than inferring domain from
  strings.

## Client construction

Existing OxiClient construction remains a connection/setup concern and may
continue producing Connection directly.

Do not route global-client lifecycle errors through the operation classifier.

## Source preservation

Every remapped mongodb::error::Error must remain available through:

  std::error::Error::source()

and must still permit downcasting to:

  mongodb::error::Error

where it does today.

Do not stringify or otherwise flatten driver errors.

Duplicate-key code 11000 must remain recoverable from the original driver
error.

## Context and Display

Preserve existing operation-context messages wherever practical.

Variant selection and human-readable operation context are separate concerns.

Display prefixes may necessarily change when the variant changes.

Public documentation must state that Display text is not the machine
classification API.

## Retry semantics

SR-2 introduces NO retry policy.

Documentation must NOT claim that Connection means the operation never reached
MongoDB or is automatically safe to retry.

If retry behavior is mentioned at all, state that retry safety depends on the
specific operation, idempotency, and application policy.

Do not add automatic retry/backoff behavior.

## Required behavior corrections

At minimum:

- duplicate key via save and update -> Database;
- client BSON serialization failure via save -> Serialization;
- unreachable-server failures across save/find/update/delete/count/exists/
  typed-query execution -> Connection;
- BSON deserialization through find_by_id, query().first(), and query().all()
  -> Serialization;
- non-connectivity index conflict -> Index;
- index connectivity failure -> Connection.

Existing GlobalClient*, Validation, Query, Custom, and non-driver Database
behavior must remain intact unless the approved classifier necessarily applies.

## Regression requirements

Implement the D5 RM-1 through RM-10 matrix.

The same underlying failure must be exercised through multiple public call
sites for the key classes:

- duplicate key;
- unreachable server;
- BSON deserialization.

For classifier unit coverage, test every classification family and as many
representative CURRENT ErrorKind variants as can be soundly constructed
without production-code distortion or unsafe/private-driver internals.

Do NOT make production APIs or test-only backdoors merely to instantiate every
ErrorKind.

The non-exhaustive/fallback policy must still be structurally present and
reviewed.

Use short server-selection timeouts for unreachable-server integration tests.

## Documentation requirements

Update OxiModError variant documentation to the approved contract.

Add user-facing error-contract guidance and migration guidance.

Explicitly document:

- failure-class rather than call-site semantics;
- Connection has no delivery/retry-safety guarantee;
- Database is the conservative non-connectivity operational fallback;
- Serialization covers both encode and decode;
- Index/Aggregation apply only after Connection/Serialization precedence;
- source() preserves driver detail;
- Display strings are not classification APIs;
- variant matchers written for 0.3.0 may observe different arms after this
  remediation.

Do not encourage users to call hidden classifier infrastructure.

## Compatibility

Acknowledge the behavioral compatibility changes from D5:

- save duplicate: Connection -> Database;
- save serialization: Connection -> Serialization;
- non-save outage: Database -> Connection;
- index outage: Index -> Connection;
- find_by_id / first deserialization: Database -> Serialization;
- Display prefix changes associated with remapped variants.

No public signature or enum-shape change is authorized.

No persisted BSON, validation timing, index lifecycle, query semantics, or hook
ordering change is authorized.

## Scope fence

Implement exactly SR-2.

Do NOT reopen SR-13.

Do NOT add _id enrichment to deserialization errors.

Do NOT implement any P-1 through P-12 item.

Do NOT add:

- retry/backoff policies;
- duplicate-key convenience APIs;
- transactions/sessions;
- typed aggregation;
- bulk-write capability;
- new query features;
- validation behavior;
- index-lifecycle behavior.

No new dependency.
No intentional Cargo.lock change.
No version bump.
No unrelated refactoring.

## Verification

Run at minimum:

  cargo fmt --all -- --check
  cargo test -p oximod_core
  cargo test -p oximod_macros
  cargo test -p oximod --test compile_fail

Run the complete new SR-2 regression suite against MongoDB.

Run:

  cargo test --doc --workspace

Run:

  cargo clippy --workspace --all-targets --all-features -- -D warnings

Then full nextest while preserving the true exit:

  set -o pipefail
  MONGODB_URI="mongodb://127.0.0.1:27019/?replicaSet=rs0" \
    cargo nextest run 2>&1 | tee /tmp/oximod-d6-nextest.log
  NEXTEST_EXIT=${PIPESTATUS[0]}
  echo "NEXTEST_EXIT=$NEXTEST_EXIT"

Do not hide or reinterpret a failing exit.

Finally:

  git status --short
  git diff --name-only
  git diff --check

Review the entire diff.

## External reverification

SR-2 is code-affecting and MUST undergo source-hidden differential external
re-verification before closure.

Primary audit target:

  W1-V07

The verifier must compare crates.io oximod = "=0.3.0" with the candidate and
must prove:

- expected variant remaps;
- unchanged mappings remain unchanged;
- identical failure classes now classify consistently across call sites;
- source() continues to downcast to mongodb::error::Error;
- duplicate-key code 11000 remains recoverable.

## Repository discipline

Do not commit.
Do not push.
Do not perform Git history operations.

Do not modify the frozen audit.

Stop after implementation, regression coverage, documentation, verification,
and complete diff review.

Do NOT mark SR-2 CLOSED.
Do NOT start release preparation.

Return the full completion report and wait for maintainer review.
