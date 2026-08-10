# OxiMod Audit Remediation — D4 Authorization

## Status

D4 AUTHORIZED — SR-3 WRITE-INDEPENDENT INDEX INITIALIZATION

## Governing documents

All existing remediation controls remain binding, including:

- REMEDIATION_BASELINE.md
- REMEDIATION_LEDGER.md
- REMEDIATION_MANDATE.md
- REMEDIATION_DIAGNOSIS.md
- REMEDIATION_D1_REVERIFY.md
- REMEDIATION_D2_AUTHORIZATION.md
- REMEDIATION_D3_AUTHORIZATION.md
- REMEDIATION_D3_REVERIFY.md

The frozen black-box audit remains immutable.

## D4 scope

D4 resolves exactly:

- SR-3

No other SR item and no P-item is authorized.

SR-3 maintainer decision:

  FIX_AND_DOCUMENT_PRE_1_0

## Goal

Provide an explicit, write-independent way for an application to establish
the indexes declared by an OxiMod collection model.

The approved public API names are:

  init_indexes()
  init_indexes_from(...)

These methods exist so applications can establish declared indexes during
startup rather than depending on the first save() to trigger index creation.

## Public API contract

Generate the new methods on COLLECTION MODELS only.

Do not generate them on embedded models.

The intended public shapes are equivalent in substance to:

  pub async fn init_indexes() -> Result<(), OxiModError>

and:

  pub async fn init_indexes_from(
      client: &mongodb::Client
  ) -> Result<(), OxiModError>

Follow the repository's existing generated-method conventions if the precise
type spelling/import path differs.

`init_indexes()` uses the global client in the same manner as existing
global-client model operations.

`init_indexes_from(...)` uses the explicitly supplied MongoDB client and must
not require global-client initialization.

Do not introduce `ensure_indexes`, `sync_indexes`, or additional aliases.

## Implementation semantics

Reuse the existing index-establishment machinery used by save paths.

Do not duplicate index-spec generation.

Do not create a second independent index lifecycle.

The explicit method and the existing save-triggered path must share the same
once-per-process establishment state for a model.

Calling `init_indexes()` before any save must establish the model's declared
indexes without inserting or updating a model document.

Existing save behavior remains valid: applications that never call
`init_indexes*()` continue to receive the existing lazy save-triggered
establishment behavior.

A successful establishment remains once-per-process under the existing
lifecycle.

Repeated successful initialization must therefore be harmless and must not
create a new synchronization/drift-checking lifecycle.

Preserve the existing retry semantics of the underlying once-initialization
mechanism. In particular, do not accidentally convert a failed establishment
attempt into a permanently-successful or permanently-poisoned state if the
existing save path permits a later retry.

## Explicit non-goals

D4 does NOT implement:

- continuous index drift detection;
- periodic index verification;
- automatic index re-establishment after an index is dropped externally;
- synchronization between declared indexes and all server indexes;
- destructive dropping or replacement of unexpected indexes;
- compound-index support;
- partial/filtered-index support;
- multi-field text-index support;
- index migrations;
- a read-only "verify but do not establish" API.

W1-F05 drift/re-establishment behavior remains outside the pre-1.0
implementation scope.

Do not broaden SR-3 into an index-management subsystem.

## Interaction with SR-7

SR-7 remains CLOSED.

The new explicit initialization path should naturally surface the same
existing index-creation failures that the save path would surface.

Do not redesign SR-7 diagnostics.

Do not add new conflict-analysis rules.

Do not change OxiModError variant meanings.

A server-side index establishment failure through `init_indexes*()` should
continue to use the existing OxiMod index-error path.

## Documentation

Document the lifecycle accurately where `#[index(...)]` is introduced and in
the README index section.

The documentation must make these points clear:

1. Declared indexes are not established merely by deriving a model.
2. Existing save operations lazily establish them.
3. Applications that need indexes before the first write can call
   `Model::init_indexes()` at startup.
4. Applications using an explicit client can call
   `Model::init_indexes_from(&client)`.
5. Successful initialization follows the existing once-per-process lifecycle.
6. This API is establishment, not continuous drift synchronization.
7. Dropping/changing indexes externally after successful initialization is
   not automatically repaired during the same process.

Do not imply that `init_indexes()` provides permanent server-state
verification.

Do not reopen unrelated D2 documentation.

## Regression requirements

Add focused permanent regressions.

At minimum prove:

A. Save-free establishment

- use a fresh collection model with at least one declared index;
- do NOT call save();
- call `init_indexes()` or `init_indexes_from(...)`;
- inspect server indexes through the MongoDB driver;
- prove the declared index exists;
- prove the collection contains zero model documents.

B. Declared specification fidelity

For representative existing supported options, prove the explicit path
establishes the same server-side specification as the save-triggered path.

Do not duplicate the entire existing index matrix if existing tests already
pin index specification generation.

Prefer a focused representative test plus reuse of existing tests.

C. Unique enforcement before first OxiMod save

Using a model with a unique declared index:

- establish indexes explicitly before any OxiMod save;
- create suitable server state without triggering OxiMod save-based index
  initialization;
- prove a conflicting typed update or other authorized existing write path is
  rejected by MongoDB because the unique index is already active.

The purpose is to prove that an application can rely on the constraint before
its first OxiMod save.

D. Idempotent successful call

- call init_indexes*() twice for the same model/process;
- both calls should succeed;
- no duplicate-index failure should result;
- semantics must remain compatible with the existing once-per-process
  lifecycle.

E. Failure surface

Using a failure scenario that is valid and isolated, prove an establishment
failure is returned through the existing index-error surface.

Do not invent a new error variant.

Do not weaken or reopen SR-7 compile-time checks merely to manufacture an
invalid model.

If an SR-7-invalid declaration can no longer compile, use another legitimate
server-side establishment-failure scenario.

F. Explicit-client API

Prove `init_indexes_from(&client)` establishes indexes through the supplied
client.

Where practical, use a process/test shape that does not depend on a global
client so this actually verifies the explicit-client contract.

G. Existing save path

Preserve existing save-triggered index tests unchanged unless a mechanical
adjustment is genuinely necessary.

Existing applications must not be required to call init_indexes*().

## Test isolation

The index initialization state is process-local and once-per-model.

Use distinct model/collection types for tests when necessary to prevent one
test's successful initialization from masking another test.

Do not make tests depend on execution order.

Do not drop/recreate an already-initialized model's indexes and then expect a
second init call to recreate them; that is the explicitly deferred drift
scenario.

Avoid introducing another long TTL-deletion wait merely to test SR-3.
Server-side TTL index specification can be inspected without waiting for the
TTL monitor.

## Regression-first discipline

Where practical:

1. add a consumer/internal regression that requires init_indexes*();
2. show the pre-D4 API fails to compile or cannot perform the save-free
   lifecycle;
3. implement the API;
4. show the regression passes.

Do not use TRYBUILD=overwrite on existing baselines.

A new positive compile/runtime regression is preferable to permanently
storing a pre-D4 E0599 failure unless a compile-fail test has lasting value.

## Implementation discipline

Prefer the smallest implementation that delegates to the existing private
index-creation mechanism.

Macro changes are expected because this is generated public API.

Do not refactor unrelated derive generation.

Do not modify index declaration syntax.

Do not modify persisted BSON.

Do not modify validation.

Do not modify typed-query behavior.

Do not change error-contract semantics.

Do not add dependencies.

Do not intentionally modify Cargo.lock.

Do not bump versions.

## Explicitly out of scope

D4 must NOT implement or alter:

- SR-2;
- any CLOSED SR item;
- SR-8 operators;
- nested validation;
- pagination behavior;
- embedded BSON conversion;
- error-contract redesign;
- sessions/transactions;
- aggregation;
- bulk writes;
- projections;
- change streams;
- P-1 through P-12.

If implementation appears to require another SR or P-item, STOP and report.

## Frozen audit

The frozen audit may be read to recover SR-3 evidence, especially:

- W3-F01
- W3-V01
- W3-A9-X02
- W3-A8-B02
- W3-A6-E01
- relevant W1-F05 lifecycle/drift evidence

It must not be modified.

## Verification

Run focused tests during development.

Before declaring D4 complete, run at minimum:

- cargo fmt --all -- --check
- cargo test -p oximod_core
- cargo test -p oximod_macros
- cargo test -p oximod --test compile_fail
- directly relevant index integration tests
- cargo test --doc --workspace
- cargo clippy --workspace --all-targets --all-features -- -D warnings

Then run:

  set -o pipefail
  MONGODB_URI="mongodb://127.0.0.1:27019/?replicaSet=rs0" \
    cargo nextest run 2>&1 | tee /tmp/oximod-d4-nextest.log
  NEXTTEST_EXIT=${PIPESTATUS[0]}
  echo "NEXTTEST_EXIT=$NEXTTEST_EXIT"

Report the true exit status.

If the historical TTL-monitor environmental failure reappears, isolate and
report it rather than weakening or skipping the TTL test.

Before completion also run:

- git status --short
- git diff --name-only
- git diff --check

Review the complete diff.

## Git discipline

Do not commit.

Do not push.

Do not rebase, merge, reset, amend, cherry-pick, or alter history.

Do not modify another worktree.

## D4 stopping condition

STOP when SR-3 has:

- focused regressions;
- implementation;
- directly relevant documentation;
- repository verification.

Do not mark SR-3 CLOSED.

Do not proceed to SR-2.

SR-3 remains subject to source-hidden external re-verification before
maintainer closure.

## Completion report

Report:

1. branch;
2. starting and ending HEAD;
3. every changed/added/deleted file;
4. exact generated public signatures;
5. which model kinds receive the methods;
6. how the methods delegate to existing index machinery;
7. how once-per-process state is shared with save paths;
8. failure/retry semantics;
9. regressions added;
10. evidence of save-free establishment with zero documents;
11. server-side index specification observed;
12. unique-enforcement-before-first-save result;
13. repeated-call result;
14. explicit-client result;
15. failure-surface result;
16. existing save-path regression result;
17. documentation changes;
18. all commands and true exit statuses;
19. full nextest totals and any TTL-only environmental issue;
20. compatibility/public-API impact;
21. confirmation drift synchronization/re-establishment was excluded;
22. confirmation SR-2, CLOSED SR items, and P-items were untouched;
23. confirmation the frozen audit was untouched;
24. final git status, name-only diff, and diff-check.

Then STOP and wait for maintainer review.
