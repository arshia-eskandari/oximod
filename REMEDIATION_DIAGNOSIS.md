# OxiMod Audit Remediation — Source-Aware Diagnosis

## Status

D0 COMPLETE — ALL 13 ITEMS DIAGNOSED — AWAITING MAINTAINER DISPOSITION

## Control information

Baseline source commit:

51cdf57dce8b7a9615008a2e325e11894e64cd39

Branch:

audit-remediation

Diagnosis phase:

D0 — source-aware, diagnosis-only

Implementation authorised:

NO

## Evidence-class conventions used throughout

- **[MEASURED]** — a black-box fact recorded by the frozen audit. Nothing in
  this document alters any audit classification, severity, verdict,
  verification count, or dissent.
- **[SOURCE]** — an explanation derived from reading the remediation worktree
  at the baseline commit. The audit did not and could not observe these
  mechanisms; they are recorded here, in the remediation record only.
- **[INFERENCE]** — a conclusion this diagnosis draws that is not directly
  measured or directly read from source.
- **[RECOMMENDATION]** — a proposal. A recommendation is not an approval;
  only the maintainer moves a ledger decision out of `UNDECIDED`.

## D0 execution record

Starting HEAD:

900a7fcec01bccbb09890acedeb50ed22815ce84 (branch `audit-remediation`; worktree
clean at start)

Ending HEAD:

900a7fcec01bccbb09890acedeb50ed22815ce84 (no commits made)

Commands/tests run:

1. `git branch --show-current`, `git rev-parse HEAD`, `git status --short` —
   clean start confirmed.
2. `git diff --stat 51cdf57..HEAD` — confirms the worktree source is
   byte-identical to the audited 0.3.0 baseline; the only additions are the
   four `REMEDIATION_*.md` control files.
3. `cargo build --workspace` — success, no warnings surfaced.
4. `cargo test -p oximod_core -p oximod_macros` — all unit tests and doctests
   pass.
5. `cargo test -p oximod --test compile_fail` — trybuild UI suite: all 11
   `tests/ui/query/*.rs` baselines pass unchanged. (`TRYBUILD=overwrite` was
   never used.)
6. `MONGODB_URI="mongodb://127.0.0.1:27019/?replicaSet=rs0" cargo nextest run`
   — 738 tests run: **737 passed, 1 failed**
   (`oximod::index ttl_index_removes_expired_documents`).
7. Re-run of the single failing test — fails again, identically.
8. Ephemeral scratchpad probes (outside the worktree, nothing tracked was
   touched) against the local mongod: `ttlMonitorEnabled = true`,
   `ttlMonitorSleepSecs = 60`, but `serverStatus.metrics.ttl.passes = 0` —
   the local TTL monitor has executed **zero passes** since server start, so
   no TTL deletion can occur. The failing test itself asserts (and passed the
   assertion) that the TTL index spec reached the server correctly
   (`expire_after: 2s`, name, key) *before* its 65-second wait; my probe
   re-confirmed the server-side index options and that the document persists
   minutes later.
9. Read-only inspection of the frozen audit repository (reports, finding
   index, and case evidence for W1-B-01, W1-V07, W2-V02, W2-V03, W3-A9-B05).
   No audit file was created, modified, or regenerated.

Unexpected failures:

One: `ttl_index_removes_expired_documents`. **Classified environmental**, not
a source regression: OxiMod's index-creation half is verified correct by the
test's own pre-sleep assertions, and the deletion half depends on the local
mongod's TTL monitor, which is demonstrably performing zero passes
(`metrics.ttl.passes = 0`). No test was modified, weakened, or skipped.

Files changed during D0:

Expected: REMEDIATION_DIAGNOSIS.md only

Observed: REMEDIATION_DIAGNOSIS.md only (verified with `git status --short`,
`git diff --name-only`, `git diff --check` at completion)

---

## SR-1 — Embedded validation descent

Audit basis:

W2-F05; W2-F06; W2-V06; W2-V07; W2-A4-E02

Audit observation being explained:

[MEASURED] `#[validate]` rules declared on a `#[model(embedded)]` type are
never evaluated for a value reached through a containing model: 135/135
matrix cells (9 container shapes × 5 rule kinds × 3 call forms) returned
`Ok`; 91 violating documents durably written; the embedded type's own
inherent `validate()` works; the parent's own field-level rules fire; no
opt-in key exists (`nested`/`dive`/`each`/`items` rejected with "unknown
attribute key"). Attached: W2-F06 — a `pre_save` hook guards `save()` but not
`save_mut()`.

Source locations:

- `oximod_macros/src/helpers/model_fields.rs:114-154` — per-field attribute
  loop; validation tokens are generated **only** from that field's own
  `#[validate(...)]` arguments.
- `oximod_macros/src/validate/model_tokens.rs:30-202` — per-field check
  generation; no branch inspects the field's *type* for an embedded model.
- `oximod_macros/src/model_macro/model_token.rs:38-75` — the generated
  `ModelCore::validate()` body is exactly the concatenation of those
  per-field checks; the generated inherent `validate()` delegates to it.
- `oximod_macros/src/lib.rs:205-222` — `__oximod_insert_with_client` calls
  `<Self as ModelCore<Collection>>::validate(self)` once, on the top-level
  model only; `save_from`/`save_from_mut` route through it
  (`oximod_macros/src/model_macro/collection_model_token.rs:57-91`).
- `oximod_macros/src/parsers/validate.rs:167` — the fallthrough
  `return Err(meta.error("unknown attribute key"))` that produced the audit's
  measured rejection of `#[validate(nested)]` and the other probed opt-ins.
- `oximod_core/src/feature/hooks.rs:87-105` — `pre_save` and `pre_save_mut`
  are distinct methods with independent no-op defaults;
  `collection_model_token.rs` invokes exactly one of them per save form —
  the complete mechanism of W2-F06.

Source-derived mechanism:

[SOURCE] The generated `validate()` is a flat function assembled from the
container's own field attributes. There is no code path anywhere in the
derive or the runtime that invokes `ModelCore::<Embedded>::validate` on a
field value, whether bare, `Option`, `Vec`, or map-valued. Embedded models
*do* generate a fully working `validate()` of their own (same
`model_token.rs` path, `Embedded` mode) — it is simply never called through
a container. Descent is therefore not "broken"; it was never generated.
[SOURCE] W2-F06 follows directly from the hook token generation: `save_from`
emits only `pre_save`/`post_save`, `save_from_mut` only
`pre_save_mut`/`post_save_mut`; neither calls the other.

Existing internal coverage:

[SOURCE] `oximod/tests/validate_*.rs` (11 files) cover every rule kind on
top-level fields, including `Option` unwrapping. **No existing test asserts
descent into an embedded value** (grep across `validate_*.rs` finds no
`model(embedded)` model) — the suite is consistent with, and silent about,
the measured boundary. `oximod/tests/hooks.rs` covers hook invocation but
not the pre_save/pre_save_mut asymmetry as a validation guard.

Reproduction/explanation:

Fully explainable from source; the 135/135 acceptance is the deterministic
consequence of no descent code existing. No runtime reproduction needed.

Smallest sound resolution:

[RECOMMENDATION] Two-lane:

1. **Documentation (sufficient to discharge the audit's "smallest resolving
   outcome"):** state the boundary — "validation does not descend into
   embedded values" — at the three places users meet it (crate-root
   `## Validation` in `oximod/src/lib.rs:170-220`, README `## Validation`,
   and the `derive.Model` field-attribute docs), together with both measured
   remedies (`#[validate(custom(path))]` on the container field; a `#[hooks]`
   `pre_save` abort) **and the W2-F06 caveat that the hook remedy requires
   both `pre_save` and `pre_save_mut`**.
2. **Optional additive code:** an opt-in `#[validate(nested)]` key on fields
   whose type (or `Option<T>`/`Vec<T>` element) derives an embedded model,
   generating a call to the embedded value's generated `validate()` and
   re-prefixing returned `ValidationError.field` values as
   `container_field.inner_field` (and `container_field.<i>.inner_field` for
   vectors). Purely additive: the key is currently a compile error, so no
   existing code changes meaning.

Automatic (non-opt-in) descent is NOT recommended pre-1.0: it would change
validation timing for existing 0.3.0 consumers — documents that save today
would begin failing — a semantic break requiring migration guidance.

Resolution class:

documentation (lane 1); optionally both (lane 1 + lane 2)

Public API impact:

Lane 1: none. Lane 2: new accepted attribute key (additive);
`ValidationError.field` gains dotted-path values *only* for opted-in fields.

Semantic/backwards-compatibility impact:

Lane 1: none. Lane 2: none for existing code (the key is currently
rejected at compile time). Persisted BSON: unchanged. Validation timing:
unchanged unless a user opts in. Index lifecycle: unchanged. Error
matching: unchanged (`OxiModError::Validation` shape reused).

Dependency/version impact:

None.

Estimated scope:

Lane 1 SMALL. Lane 2 MEDIUM (parser key + token generation for
bare/Option/Vec shapes + path prefixing + tests).

Proposed internal regression:

Lane 1: doc examples compile-checked. Lane 2: new
`oximod/tests/validate_nested.rs` (bare, `Option`, `Vec` shapes; error path
attribution) plus a `tests/ui` case asserting the opt-in is accepted and a
case asserting `#[validate(nested)]` on a non-embedded type is rejected.

External re-verification:

W2-V07 validation matrix; W2-A4-E02; hook-remedy coverage (W2-F06 case) if
the hook remedy is documented.

Nearby regression risks:

Existing `Option` unwrapping semantics in generated checks
(`validate/macros.rs` `opt_check!`); flat `ValidationError.field` consumers
if dotted paths are introduced; hook ordering (`pre_save` → validate →
insert) documented at `lib.rs:173-176`.

Confidence:

HIGH

Open maintainer questions:

1. Documentation-only pre-1.0, or also ship the `#[validate(nested)]` opt-in?
2. If the opt-in ships: is dotted-path `ValidationError.field` acceptable, and
   what is the element-index format (`items.1.sku` as measured by the audit's
   hook remedy)?
3. Should automatic descent ever become the default at 1.0 (a breaking,
   flag-day decision that must be taken deliberately, not here)?

Recommended maintainer decision:

DOCUMENT_PRE_1_0 (minimum, discharges the finding) — with
FIX_AND_DOCUMENT_PRE_1_0 as the offered upgrade if the opt-in is wanted
before 1.0.

---

## SR-2 — Error variant documentation / semantics

Audit basis:

W1-F11; W1-V07

Audit observation being explained:

[MEASURED] The `OxiModError` variant identifies the call site, not the
failure class; measured against the published variant docs the meanings of
`Connection` and `Database` are inverted for the two most consequential
failures (duplicate key from `save` → `Connection`; unreachable server from
five non-save methods → `Database`; deserialization failure → `Database`
from `find_by_id`, `Serialization` from `query().all()`). The audit's only
`confirmed defect (documented behaviour contradicted)`.

Source locations:

- `oximod_macros/src/lib.rs:224-232` — **the single line producing the
  headline inversion**: every `insert_one` failure inside
  `__oximod_insert_with_client` is wrapped as
  `OxiModError::connection("Failed to insert document into MongoDB collection", error)`
  — duplicate keys, validation-free server rejections, and genuine outages
  alike.
- `oximod_macros/src/model_macro/collection_model_token.rs:105-207` —
  `find_by_id`/`delete_by_id`/`update_by_id`/`clear` wrap **all** their
  driver errors (including unreachable-server) as `OxiModError::database(...)`.
- `oximod_core/src/feature/model.rs:389-420` — `exists`/`count` likewise
  `database(...)`.
- `oximod_core/src/query/builder.rs:552-674` — typed query execution wraps
  driver errors as `database(...)` except the cursor deserialization step
  (`builder.rs:666-668`), which is `serialization(...)` — the measured
  `find_by_id` vs `all()` divergence for the identical failure class.
- `oximod_core/src/error/oximod_error.rs:77-91, 113-126, 172-187` — the
  published variant doc comments the audit measured against ("Failed to
  connect to the MongoDB server", "when a database operation fails ...
  insert, update, delete, find", serialization "mismatched BSON types ...").

Source-derived mechanism:

[SOURCE] Each generated call site picks one fixed constructor for every
error the driver returns from that operation. The variant therefore encodes
"which OxiMod method was executing", while the docs describe failure
classes. `find_by_id`'s wrapping also explains the W1-F12/W1-F13-adjacent
"Failed to find document by _id" outer message on a document that exists but
cannot be deserialized, and the SR-13 evidence's I3 rows.

Existing internal coverage:

[SOURCE] No internal test asserts which variant a duplicate key or an
unreachable server maps to. `oximod_core` unit tests cover
`QueryError`→`OxiModError` conversion only.

Reproduction/explanation:

Fully explainable from source; every measured mapping in W1-V07's matrix
corresponds line-for-line to a call-site constructor above.

Smallest sound resolution:

[RECOMMENDATION] Lane A (documentation, smallest): rewrite the variant doc
comments in `oximod_error.rs` to state what the variants actually identify
(the operation being attempted, not the failure class), and document
`source()` + `downcast_ref::<mongodb::error::Error>()` as the supported
classification route with the audit's 7-line duplicate-key predicate as the
worked example. Lane B (code, still small but semantic): change the insert
path's mapping from `connection` to `database` (one line in
`oximod_macros/src/lib.rs`), removing the inversion while keeping call-site
semantics; a full failure-class classifier (inspecting
`mongodb::error::ErrorKind` in a shared helper, e.g.
`OxiModError::from_driver(msg, error)`) is the complete fix but changes
error matching across the whole surface (MEDIUM).

Resolution class:

documentation (lane A); both if lane B is approved

Public API impact:

Lane A none. Lane B: no signature change; `Display` messages and variant
selection change.

Semantic/backwards-compatibility impact:

Lane A: none. Lane B (insert remap): 0.3.0 code matching
`OxiModError::Connection` around `save()` changes behaviour — **error
matching changes: YES; migration guidance required**. Persisted BSON,
validation timing, index lifecycle: unchanged in both lanes.

Dependency/version impact:

None (the classifier would name `mongodb::error::ErrorKind` internally,
already a direct dependency).

Estimated scope:

Lane A SMALL. Lane B one-line remap SMALL; full classifier MEDIUM.

Proposed internal regression:

New `oximod/tests/error_classification.rs`: induced duplicate key (unique
index + second save) asserting the documented variant; deserialization
failure via raw-inserted malformed doc asserting `find_by_id`/`all()`
variants — written to whichever contract the maintainer chooses.

External re-verification:

W1-V07 error-classification cases.

Nearby regression risks:

Any consumer/test matching on variant or `Display` text of save-path errors;
`examples/validate_extract_errors.rs`-style error handling; SR-13's
recommended message enrichment (below) touches the same call sites.

Confidence:

HIGH

Open maintainer questions:

1. Is the pre-1.0 contract "variants identify the operation" (fix docs) or
   "variants identify the failure class" (fix mapping)? This decision gates
   the regression test's assertions.
2. If mapping changes: insert-line-only remap, or full classifier?

Recommended maintainer decision:

FIX_AND_DOCUMENT_PRE_1_0 — lane A doc correction in all cases, plus the
one-line insert remap (`connection` → `database`) as the smallest code change
that removes the measured inversion; full classifier deferred unless the
maintainer wants the failure-class contract before 1.0.

---

## SR-3 — Index establishment lifecycle / explicit establish-or-verify path

Audit basis:

W3-F01; W3-V01; W3-A6-E01; W3-A8-B02; W3-A9-X02

Audit observation being explained:

[MEASURED] Of twelve public call forms, exactly one — `save()` — establishes
a model's declared `#[index]` specs; save()-free write mixes run with
declared unique/TTL constraints silently absent; the typed update path can
violate a declared unique invariant while returning `Ok`; one throwaway
`save()` per collection establishes everything verbatim. The crate-root
sentences about the trigger were measured accurate in both directions.
(Adjacent, W1-F05: a successful establishment is remembered for the process
lifetime and never re-checked.)

Source locations:

- `oximod_macros/src/lib.rs:153-222` — per-model
  `static _INDEX_INIT_{Model}: OnceAsync` plus the generated
  `_create_indexes()`; **the only call site is
  `__oximod_insert_with_client`** (line 222), reached solely from
  `save_from`/`save_from_mut`.
- `oximod_macros/src/model_macro/collection_model_token.rs` — none of
  `find_by_id_from`, `update_by_id_from`, `delete_by_id_from`, `clear_from`,
  `get_collection_from` touch `_create_indexes`.
- `oximod_core/src/query/builder.rs` — no typed execution method touches
  index initialization.
- `oximod_core/src/helpers/once_async.rs:237-282` — success permanently
  completes the initializer (explains W1-F05's no-re-establishment); failure
  resets state so a later save retries (matches the documented "a later save
  can try again"); the retry/timeout options are explicitly diagnostic-only
  (`once_async.rs:8-12`, honestly documented at `oximod/src/lib.rs:636-638`).
- Doc statements: `oximod/src/lib.rs:228-234` and `README.md:988-990` are the
  accurate trigger sentences; the `derive.Model` **field-attribute list**
  (`oximod/src/lib.rs:654-658`) mentions `#[index(...)]` with no trigger
  note — the placement gap the audit's smallest outcome names.

Source-derived mechanism:

[SOURCE] Index establishment is lazily coupled to the insert path and to
nothing else, guarded per model type per process. Every audited behaviour —
save-only establishment, per-model-type scoping, first-save failure on a
poisoned declaration (SR-7), retry-after-failure, no re-establishment after
out-of-band drops — is the direct consequence of these ~70 generated lines
plus `OnceAsync` semantics. There is no public write-independent
establish/verify entry point; `_create_indexes` is `#[doc(hidden)]` inherent
and takes the typed collection as an argument.

Existing internal coverage:

[SOURCE] `oximod/tests/index.rs` verifies option forwarding and
establishment **via save()** for many index kinds (including the TTL test
that failed environmentally here). No test covers save()-free
establishment, because no such path exists.

Reproduction/explanation:

Fully explainable from source. Not re-reproduced at runtime in D0 (would
require a save()-free harness; the audit's own verification is recent and
uncontested).

Smallest sound resolution:

[RECOMMENDATION] Two parts, matching the audit's smallest outcome exactly:

1. **Documentation placement (SMALL):** add the trigger sentence (and the
   startup-`save()`/raw-`create_index` guidance) to the `derive.Model`
   field-attribute docs where `#[index]` is introduced, and to the README
   index section's opening.
2. **Additive API (SMALL-MEDIUM, needs explicit approval as a public API
   change):** generate a public inherent
   `async fn init_indexes() -> Result<(), OxiModError>` (plus
   `init_indexes_from(&Client)`) on collection models, delegating to the
   existing `_create_indexes` machinery — a write-independent establish path
   that also *verifies* (a poisoned declaration fails here at startup instead
   of at first save, materially helping SR-7). Reusing the existing
   `OnceAsync` static keeps semantics identical to save-path establishment.

A "verify without establishing" read-only checker is larger and not the
smallest outcome; not proposed pre-1.0. Re-establishment on drift (W1-F05)
is intentionally NOT bundled: periodic re-checks would change the documented
once-per-process contract.

Resolution class:

both (doc placement + additive API), pending approval of part 2;
documentation alone if part 2 is declined

Public API impact:

Part 2 adds inherent methods to every derived collection model. Collision
risk: a user-defined inherent `init_indexes` on their model would now
conflict (compile error) — judged unlikely but it is a compile-surface
change on generated code; flagged.

Semantic/backwards-compatibility impact:

No existing behaviour changes; establishment trigger set widens only if the
user calls the new method. Persisted BSON: unchanged. Index lifecycle:
extended (new voluntary trigger), existing triggers unchanged. Error
matching: unchanged (reuses `OxiModError::Index`).

Dependency/version impact:

None.

Estimated scope:

Doc SMALL; API SMALL-MEDIUM (macro tokens + docs + tests).

Proposed internal regression:

`oximod/tests/index.rs` additions: `init_indexes()` establishes declared
specs with zero documents written (assert via `list_indexes`); unique
enforcement active for a typed update immediately after; second call is a
no-op; establishment failure surfaces `OxiModError::Index`.

External re-verification:

W3-V01 index-establishment cluster (typed-update leg, raw-ingest leg, TTL
differential), W3-A9-X02, W3-A8-B02, W3-A6-E01.

Nearby regression risks:

`OnceAsync` once-per-process semantics (shared with save path); SR-7's
first-save failure surface; the documented "later save can try again"
retry contract.

Confidence:

HIGH

Open maintainer questions:

1. Approve the additive `init_indexes()`/`init_indexes_from()` public API?
2. Method naming (`init_indexes` vs `ensure_indexes` vs `sync_indexes` — the
   audit measured probes for `init_indexes`/`sync_indexes`/`ensure_indexes`
   all E0599, so any choice is new surface).
3. Should W1-F05's drift scenario stay documented-only pre-1.0 (this
   diagnosis says yes; re-establishment would change a documented contract)?

Recommended maintainer decision:

FIX_AND_DOCUMENT_PRE_1_0 (doc placement now; `init_indexes()` as the
approved additive API).

---

## SR-4 — Derived composite-key substitute warning

Audit basis:

W1-F16; W1-B-07; W2-A3-B02; W2-A4-X03; W2-A5-X02

Audit observation being explained:

[MEASURED] Compound uniqueness is a documented raw-driver boundary
(routing sentence retrieved verbatim); the substitute readers invent — a
derived composite key field carrying `#[index(unique)]` — desynchronises
under `update_by_id`, after which a genuine duplicate pair persists while
`listIndexes` looks healthy; measured independently in three archetypes; in
A3 the desync blocked its own remediation.

Source locations:

- Routing sentences (accurate, in place): `oximod/src/lib.rs:232-234`;
  `README.md:990`, `README.md:1220`, `README.md:1262`.
- Why desync is inevitable [SOURCE]: `update_by_id_from`
  (`collection_model_token.rs:155-186`) applies the caller's raw update
  document with no knowledge of field derivations; the typed
  `update_one`/`update_all` (`builder.rs:828-949`) likewise write exactly
  the requested paths. OxiMod has no computed/derived-field concept, so
  nothing recomputes a composite field when its source fields change. This
  is inherent to storing derived data, not an implementation bug.

Existing documentation coverage:

The routing sentence exists at three sites; **no site warns that the derived
composite key is not a safe substitute**. No warning text exists anywhere in
README.md or the rustdoc (grep for "composite" returns nothing).

Smallest sound resolution:

[RECOMMENDATION] One warning sentence adjacent to each routing sentence
(README index section, README limitations table/list, crate-root Indexes
section): a derived composite key with `#[index(unique)]` is not a safe
substitute for a compound unique index — it silently desynchronises under
partial updates (`update_by_id`, typed `$set`); use a raw `create_index` at
deploy, which coexists with declared indexes.

Resolution class:

documentation

Compatibility impact:

None (no code change; no API, BSON, validation, index, or error change).

Estimated scope:

SMALL

Verification:

Documentation review; the warning must label E1 unsafe wherever compound
uniqueness is discussed (audit requirement). No new capability implied or
proposed.

Confidence:

HIGH

Open maintainer questions:

None blocking. (Whether compound `#[index]` should ever exist is a post-1.0
product question, out of scope by mandate.)

Recommended maintainer decision:

DOCUMENT_PRE_1_0

---

## SR-5 — Partial / filtered uniqueness boundary

Audit basis:

W1-F16 family; W2-A3-B02; W2-A4-X03; W2-A5-X02

Source/documentation locations:

- [SOURCE] `oximod_macros/src/parsers/index.rs:19-135` — the `#[index]` key
  allowlist contains no `partial`/`partial_filter` key; unknown keys hit
  `meta.error("unknown attribute key")` at line 130 (the audit's measured
  compile rejections).
- [SOURCE] `oximod_macros/src/index/model_tokens.rs:137-161` — the generated
  `IndexOptions` builder never sets `partial_filter_expression`, although the
  driver's `IndexOptions` supports it (confirmed present in
  `mongodb`/`bson` dependency surface). The boundary is a deliberate
  non-exposure, not an oversight in forwarding.
- Documentation: the compound-index routing sentence exists
  (`README.md:990`, `oximod/src/lib.rs:232-234`), but **no routing statement
  exists for partial/filtered indexes anywhere** (grep for "partial" across
  README.md and rustdoc: zero hits) — matching the audit's
  provenance-bounded observation that none was retrieved.

Current documented boundary:

Compound → routed to `get_collection()`. Partial → silent.

Smallest sound resolution:

[RECOMMENDATION] Extend the existing routing sentences: "partial/filtered
indexes (MongoDB `partialFilterExpression`) are likewise created through the
collection returned by `Model::get_collection`", noting (a) violations of a
raw partial-unique index surface as driver `E11000`, not
`OxiModError::Validation`, and (b) MongoDB's own grammar restriction on
`partialFilterExpression` (measured independently in A4 and A5 — a MongoDB
restriction, not OxiMod's). Exposing `partial_filter` as an `#[index]`
option would be feature expansion beyond the smallest outcome and is not
proposed; it remains available as a maintainer choice (the forwarding change
itself would be SMALL, but the option's expression grammar is a raw BSON
document — an awkward fit for the attribute surface).

Resolution class:

documentation (the capability boundary itself: intentional boundary)

Compatibility impact:

None.

Estimated scope:

SMALL

Verification:

Documentation review; raw hatch remains the boundary unless separately
approved (per ledger).

Confidence:

HIGH

Open maintainer questions:

Whether to also fence `#[index(partial_filter = ...)]` in as a post-1.0
consideration — a product decision, not needed to discharge SR-5.

Recommended maintainer decision:

DOCUMENT_PRE_1_0

---

## SR-6 — exists() / count() inconsistency

Audit basis:

W2-F02; W2-V03; W2-V04

Audit observation being explained:

[MEASURED] `Model::exists()` returns `Err` (wrapping a driver
BsonDeserialization source) where `Model::count()` returns `Ok` on the
identical predicate when the matched set contains an undeserializable
document; three raw idioms and OxiMod's own `count()` do not reproduce it.
The audit deliberately asserted no implementation cause.

Source locations:

- `oximod_core/src/feature/model.rs:389-396` — `exists_from` runs
  `find_one(filter)` **on the typed collection** (`MongoCollection<Self>`),
  so the matched document is deserialized into the model before
  `.is_some()` is evaluated; a deserialization failure surfaces as
  `Err(OxiModError::Database { msg: "Failed to check document existence", .. })`
  with the `BsonDeserialization` driver error as source.
- `oximod_core/src/feature/model.rs:414-420` — `count_from` runs
  `count_documents(filter)`, a server-side count that never deserializes.
- The rustdoc on `exists_from` (`model.rs:373-375`) already states the
  implementation idiom ("implemented using `find_one(filter).await?.is_some()`")
  but does not state the deserialization consequence.

Source-derived mechanism:

[SOURCE] The inconsistency is exactly one type parameter: `find_one` on
`Collection<Self>` versus a count (or a `Collection<Document>` read). The
audit's carefully-hedged observable difference now has a one-line cause.

Existing internal coverage:

`oximod/tests/exists.rs` exists and covers well-formed data only; no test
exercises a malformed matched document.

Reproduction/explanation:

Fully explainable from source; deterministic.

Smallest sound resolution:

[RECOMMENDATION] Change `exists_from` to probe through the raw document
collection:
`Self::get_document_collection_from(client)?.find_one(filter)` (optionally
with a `{_id: 1}` projection) — one line, sibling agreement restored,
`exists()` becomes a true cheap existence probe. Doc-only alternative:
document that `exists()` deserializes and is not equivalent to
`count() > 0` over possibly-malformed data.

Resolution class:

code (with a one-sentence doc touch to the already-explicit implementation
note)

Public API impact:

None (signature unchanged).

Semantic/backwards-compatibility impact:

Behaviour change in exactly the audited corner: `exists()` over a matched
undeserializable document changes `Err` → `Ok(true)`. **Error matching
changes: YES, narrowly** — code that relied on `exists()` to surface
deserialization corruption (an idiom the audit classifies as a hazard, not a
feature) would stop erring. No compile breakage; persisted BSON, validation
timing, index lifecycle unchanged. Migration note: one sentence in the
changelog.

Dependency/version impact:

None.

Estimated scope:

SMALL

Proposed internal regression:

`oximod/tests/exists.rs` addition: raw-insert a document malformed for the
model, assert `exists(filter)` = `Ok(true)` and agreement with
`count(filter) > 0` (assertions written to the maintainer-chosen contract;
if doc-only is chosen instead, the test pins the current `Err`).

External re-verification:

W2-V03 and W2-V04 malformed-document probes.

Nearby regression risks:

`find_one`-based semantics elsewhere (`first()`, `find_by_id`) must NOT
change — they intentionally return the typed model; only the boolean probe
changes. Server-version count behaviour is unaffected.

Confidence:

HIGH

Open maintainer questions:

Accept the narrow `Err`→`Ok(true)` behaviour change pre-1.0? (This diagnosis
says yes: the audit's user consequence is precisely that `exists()` should be
a safe cheap probe.)

Recommended maintainer decision:

FIX_PRE_1_0

---

## SR-7 — Conflicting index declarations need an earlier signal

Audit basis:

W2-F07; W2-V08; W2-A4-E01; W2-A3-B02

Audit observation being explained:

[MEASURED] Two `#[index(text)]` fields, or two fields sharing one
`#[index(name = ...)]` literal, compile with zero diagnostics; every save()
for that model type then fails with server code 85/86 wrapped in a generic
`OxiModError::Index` Display naming no collection, field, code, or remedy;
fails closed, recovers on next save after the attribute is removed; both
conditions decidable from the declaration alone.

Source locations:

- `oximod_macros/src/parsers/index.rs` — strictly per-attribute parsing; its
  module doc (lines 10-12) defers "compatibility checks between index types
  and their options" to the generation layer.
- `oximod_macros/src/index/model_tokens.rs` — per-field token generation;
  `generate_key_entry` (lines 165-188) computes the text-implying predicate
  (`text` flag OR any of `text_index_version`/`default_language`/
  `language_override`/`weight`) — the exact predicate a cross-field check
  needs already exists here. **No cross-field view exists anywhere**:
  `generate_field_tokens` (`helpers/model_fields.rs:114-131`) converts each
  `IndexArgs` to tokens immediately and discards the parsed args.
- Runtime failure surface: `oximod_macros/src/lib.rs:183-197` — the
  generated `create_indexes` call maps every server rejection to
  `OxiModError::index("Failed to create indexes for collection", error)`;
  the static message is why nothing is named (the collection literal `#db`/
  `#collection` is in scope at generation time and could be interpolated).
- Per-model-type scoping and next-save retry: the `_INDEX_INIT_{Model}`
  `OnceAsync` (SR-3) — matching the audit's refutations of "permanent" and
  "collection-wide".

Source-derived mechanism:

[SOURCE] Nothing validates the *set* of declared indexes; the first save
ships the full `Vec<IndexModel>` to the server, whose all-or-nothing
`createIndexes` rejects (85: two text indexes; 86: duplicate name), and
`OnceAsync` failure-reset yields the measured retry behaviour.

Existing internal/UI coverage:

`oximod/tests/index.rs` covers valid single declarations;
`oximod/tests/ui/` has no index-declaration compile-fail cases. No test
declares a conflicting pair.

Reproduction/explanation:

Fully explainable from source; not re-run at runtime in D0.

Smallest sound resolution:

[RECOMMENDATION] Two independent parts:

1. **Compile-time cross-field check (the "earlier signal"):** collect
   `(field, IndexArgs)` pairs in `generate_field_tokens` before token
   conversion; emit `compile_error!` when (a) more than one field is
   text-implying (per the existing `generate_key_entry` predicate — MongoDB
   allows one text index per collection), or (b) two `#[index]` attributes
   carry the same `name` literal. Both are exactly the two conditions the
   audit proved decidable from the declaration. Residual, documented: two
   *different models* sharing one collection can still conflict — not
   decidable from a single declaration; out of scope.
2. **Runtime message enrichment:** interpolate the collection name into the
   generated index-error message ("Failed to create indexes for collection
   `{collection}`"), leaving the server detail reachable via
   `source()`/`{:?}` as today.

Resolution class:

code

Public API impact:

None (no signatures). Compile surface: previously-compiling poisoned
declarations become compile errors.

Semantic/backwards-compatibility impact:

**Compile breakage: YES, deliberately and narrowly** — only declarations
that today compile and then deterministically fail every `save()` at runtime
(the audit measured zero-footprint fail-closed behaviour, so no working
deployment can contain one... with one caveat: a model whose conflicting
declaration is *never saved* in practice compiles today and would stop
compiling; flagged for the maintainer). Display text of the index error
changes (part 2) — error matching on the exact message string would break;
variant unchanged. Persisted BSON, validation timing, established-index
lifecycle: unchanged.

Dependency/version impact:

None.

Estimated scope:

Part 1 SMALL-MEDIUM (refactor of `generate_field_tokens` to two passes +
two checks + UI tests). Part 2 SMALL.

Proposed internal/UI regression:

New `oximod/tests/ui/index/two_text_indexes.rs` and
`duplicate_index_name.rs` compile-fail cases with baselines; positive
control: existing `index.rs` suite keeps compiling (single text index case
already covered there).

External re-verification:

W2-A4-E01 / W2-V08 declaration matrix; W2-A3-B02 DUPNAME_PROBE (code-86
half).

Nearby regression risks:

Legitimate multi-index models (several scalar indexes with distinct names);
text-implying inference (a `weight`-only attribute is text-implying — the
check must use the same predicate as generation or the two layers diverge);
the documented retry-on-next-save contract (unchanged by part 1, since
conflicts no longer reach runtime).

Confidence:

HIGH

Open maintainer questions:

1. Approve the compile-surface change (a strictly-protective break)?
2. Should the duplicate-name check also cover the implicit case (two
   `#[index(text)]` without names produce distinct server names — yes,
   covered by the text check; scalar unnamed indexes get distinct
   field-derived names — no conflict; only literal `name =` duplicates are
   checkable, which is what the audit measured)?

Recommended maintainer decision:

FIX_PRE_1_0 (both parts).

---

## SR-8 — Option<T> ordered/numeric operator boundary

Audit basis:

W2-F01; W2-V01; W2-A7-B02; W2-A3-B01

Audit observation being explained:

[MEASURED] Ordered (`gt/gte/lt/lte`), numeric (`inc/mul/min/max/modulo`) and
bitwise operators are rejected on `Option<T>` fields for every probed inner
type and spelling, while equality/membership/$set, exists/is_null, asc/desc,
regex on `Option<String>`, and `current_date()` on `Option<DateTime>` are
Option-transparent — four families transparent, three not.

Source locations:

- `oximod_core/src/query/field/traits.rs` — the entire asymmetry:
  `OrderedQueryValue` (16-20), `NumericQueryValue` (38-40), and
  `IntegerQueryValue` (54-55) have **no `Option<T>` impls**, while
  `StringQueryValue` (27-28) and `DateQueryValue` (47-48) each explicitly
  add one. The measured "four transparent, three excluded" split maps
  one-to-one onto which marker traits received an `Option` impl.
- `oximod_core/src/query/field/scalar.rs:52-149` — `eq/ne/in/nin/set` are
  bounded on `T: Into<Bson>` only; `bson` 2.15 provides
  `impl<T: Into<Bson>> From<Option<T>> for Bson` (verified at
  `bson-2.15.0/src/bson.rs:451`), which is why equality is transparent.
- `oximod_core/src/query/field/scalar.rs:151-187` (`gt/gte/lt/lte`) and
  `field/numeric.rs:12-131` (`inc/mul/min/max/modulo`, bitwise) — bounded on
  the marker traits above; `Field<Option<i64>>` fails the bound, producing
  the measured E0277s.
- Option-only surface: `scalar.rs:12-50` (`is_null`/`unset`/`rename_to`),
  `field.rs:123-151` (`asc`/`desc`/`exists`).

Source-derived mechanism:

[SOURCE] The boundary is a set of missing trait impls, not a structural
constraint. Type-level feasibility of the fix is confirmed: with bson's
`From<Option<T>>`, either (a) blanket
`impl<T: OrderedQueryValue> OrderedQueryValue for Option<T>` (and numeric/
integer analogues) or (b) dedicated inherent methods on `Field<Option<T>>`
taking the **inner** type (`V: Into<T>`) compile cleanly. [INFERENCE]
Option (a) has a semantic hazard: `field.gt(None)` would compile and emit
`{"$gt": null}`, and `inc(None)` would emit `$inc: null` (a server error at
runtime) — likely why the impls are absent. Option (b) forecloses `None`
arguments entirely and matches the builder-setter convention (inner type,
never Option-typed) established by F-V01.

Existing internal/UI coverage:

`oximod/tests/ui/query/ordered_comparison_on_boolean.rs` pins ordered-op
rejection on `bool`; **no UI baseline pins the `Option<T>` rejection**, so
exposing the operators breaks no existing baseline (confirmed by the passing
trybuild run). `field_queries.rs` covers bare-type operators.

Reproduction/explanation:

Fully explainable from source; the audit's compile matrix corresponds
exactly to the trait-impl table.

Smallest sound resolution:

[RECOMMENDATION] Code lane: dedicated inherent impls on `Field<Option<T>>`
for ordered ops (`T: OrderedQueryValue + Into<Bson>`, args `V: Into<T>`),
and equivalents for the numeric/modulo family; bitwise likewise if desired.
Documentation lane (audit-sanctioned alternative): document the boundary and
the `get_collection()` + `Queryable::fields()` + `Field::name()` workaround
(~3 lines, rename-safe) where the operators are documented.

Resolution class:

code (documentation lane acceptable if the maintainer prefers zero surface
change pre-1.0)

Public API impact:

Purely additive methods on `Field<Option<T>>`.

Semantic/backwards-compatibility impact:

No existing code changes meaning (currently E0277); no persisted BSON,
validation, index, or error-matching change. New queries express `$gt` etc.
against possibly-null fields — MongoDB semantics of ordered comparison
against null/missing apply and should be one documentation sentence.

Dependency/version impact:

None (uses existing bson `From<Option<T>>`).

Estimated scope:

SMALL-MEDIUM (impl blocks + docs + tests; no macro changes needed).

Proposed internal regression:

`oximod/tests/field_queries.rs` additions (Option<i64>/Option<DateTime>
range query round-trips; `Option<i32>` `$inc` via `update_one`); expression
unit tests in `oximod_core` mirroring the existing scalar tests.

External re-verification:

W2-V01 operator compile matrix (expected flips rejected→accepted);
W2-A7-B02, W2-A3-B01 workflow probes.

Nearby regression risks:

The audited-coherent builder/setter conventions (F-V01) — inner-type
argument convention must be kept consistent; `is_null`/`exists` semantics
unchanged; no accidental blanket impl leaking `gt(None)`.

Confidence:

HIGH

Open maintainer questions:

1. Code lane or documentation lane pre-1.0?
2. If code: inner-type-argument design (recommended) confirmed? Include the
   bitwise/integer family or just ordered+numeric (the audit's measured
   workflows needed ordered and `$inc`)?

Recommended maintainer decision:

FIX_PRE_1_0 (inner-type inherent methods, ordered + numeric families;
bitwise optional).

---

## SR-9 — elem_match / Vec<Embedded> operator ergonomics

Audit basis:

W2-F03; W2-V02; W2-A4-B03

Audit observation being explained:

[MEASURED] `push`/`add_to_set`/`pull`/whole-array `set` are rejected on
`Vec<Embedded>` (E0599, `Into<Bson>` unsatisfied); a three-line consumer
`impl From<MyEmbedded> for Bson` makes all four compile with byte-identical
stored BSON; "elem_match is separately rejected in five spellings";
`.positional()` compiles but was not usable end to end because no typed
array-matching filter clause could be emitted.

Source locations and source-derived mechanism:

- Mutation half [SOURCE]: `oximod_core/src/query/field/array.rs:13-94` — all
  mutation methods sit in `impl<T> Field<Vec<T>> where T: Into<Bson>`;
  embedded models do not implement `Into<Bson>`; whole-array `set`
  (`scalar.rs:52-55, 134-139`) needs `Vec<T>: Into<Bson>`, same root. The
  consumer `From` impl satisfies the existing bounds — exactly the measured
  three-line remedy.
- **elem_match half — material source-aware correction [SOURCE]:**
  `oximod_core/src/query/field/embedded.rs:40-52` defines
  **`elem_match_nested`** on `Field<Vec<T>> where T: FieldSchema` — a typed
  `$elemMatch` over arrays of embedded models — **present at the audited
  baseline commit** (verified via `git show 51cdf57:...`), documented on the
  docs.rs crate root (the very block 7 flagged by SR-11), shown in
  `README.md:591-596`, exercised by internal tests
  (`oximod/tests/embedded_queries.rs:276`, `oximod/tests/array_updates.rs:353-357`
  — the latter is precisely the `elem_match_nested` filter +
  `positional()` update pair end-to-end) and by
  `examples/typed_query.rs:436`. The same impl block provides `filtered()` +
  `Query::array_filter` (`embedded.rs:93-135`, `builder.rs:504-514`) as the
  `$[ident]` alternative.
- Why the audit measured it unreachable [MEASURED + SOURCE]: the audit's
  probe named `p15_emb_elem_match_nested.rs` actually called
  `.elem_match(|e| e.nested(...))` — composing two real methods, not calling
  `elem_match_nested` itself (`evidence/W2-V02/compile-probe.txt:302-313`);
  none of the five spellings was the direct `elem_match_nested(...)` call.
  The five rejections are all genuine compile facts and stand as measured.
  [INFERENCE] The audit's *reachability conclusion* was materially caused by
  the SR-11 documentation gap: the one crate-root block demonstrating
  `elem_match_nested` references undeclared fields, so the audit could not
  compile it and never confirmed the method's signature (their own W1-B-01
  note records exactly this). This does not retroactively alter W2-F03's
  classification; it is a remediation-record explanation.
- `.positional()` dead-end [SOURCE]: `embedded.rs:78-86` — positional
  requires an array-matching query clause; `elem_match_nested` emits exactly
  that clause; the internal test proves the pair works. The audit's two
  attempted filters (`_id`, scalar sibling) correctly could not satisfy `$`.

Existing internal/UI coverage:

`embedded_queries.rs`, `array_updates.rs` (elem_match_nested, positional,
filtered/array_filter, end-to-end against MongoDB — all passing in this
D0 run); `tests/ui/query/elem_match_on_scalar_field.rs` and
`nested_elem_match_on_scalar_array.rs` pin the scalar-side compile surface.
No internal coverage for the mutation-half `From<Embedded> for Bson` remedy.

Reproduction/explanation:

Fully explainable; the internal suite already demonstrates the capability
the audit could not reach.

Smallest sound resolution:

[RECOMMENDATION] Documentation-first:

1. Fix the crate-root block that demonstrates `elem_match_nested` so it is
   self-contained and compiles (shared work with SR-11) — this is the exact
   discoverability failure that produced the audit's dead end.
2. Document the three-line `impl From<MyEmbedded> for Bson` remedy at the
   array-operator docs (`array.rs` method docs and README arrays section),
   plus the `elem_match_nested`→`positional` pairing requirement on
   `positional()`'s doc (it is stated at `embedded.rs:56-58` but deserves
   the pairing example — which the doc example at `embedded.rs:60-77`
   already shows; surface it in README too).
3. Optional additive code (maintainer decision): derive-generate
   `impl From<#name> for Bson` for embedded models so the mutation operators
   work out of the box. Hazard to weigh: `From` must not fail;
   `bson::to_bson` can error, so the generated impl would have to panic on
   serialization failure (or the operators gain new fallible variants) —
   a real design decision, not obviously worth it pre-1.0 given the
   three-line consumer remedy.

Resolution class:

documentation (primary); optional additive code for the mutation half

Public API impact:

Doc: none. Optional code: a generated `From<Embedded> for Bson` impl —
additive, but occupies the impl slot a consumer may have written (coherence:
a consumer's own `impl From<MyEmbedded> for Bson` would now conflict —
**this makes the optional code change breaking for exactly the users who
applied the audit's remedy**; flagged prominently).

Semantic/backwards-compatibility impact:

Doc: none. Optional code: compile breakage for existing consumer `From`
impls (above); panic-on-serialization-failure semantics inside `From`.

Dependency/version impact:

None.

Estimated scope:

Doc SMALL. Optional generated impl SMALL-MEDIUM (with the breaking caveat).

Proposed internal regression:

Doc-lane: compiled doc examples (SR-11 work). If the generated `From`
ships: `oximod/tests/array_updates.rs` additions exercising
`push`/`pull`/`add_to_set` on `Vec<Embedded>` without a hand-written impl.

External re-verification:

W2-V02 array/operator matrix **extended with the direct
`elem_match_nested(...)` spelling**; W2-A4-B03 moderation-queue workflow.

Nearby regression risks:

Existing consumer `From<Embedded> for Bson` impls (if the generated impl
ships); the scalar `elem_match` surface (must stay unchanged); stored-BSON
byte-compatibility of any generated conversion (must use `to_bson` with the
model's serde attributes, as the consumer remedy does).

Confidence:

HIGH

Open maintainer questions:

1. Accept documentation-first, deferring any generated `From` impl?
2. If a code path for mutation ops is wanted pre-1.0, prefer generated
   `From` (breaking for remedy-users, panics on serialize failure) or new
   fallible method variants (more surface)? This diagnosis recommends
   neither pre-1.0.

Recommended maintainer decision:

DOCUMENT_PRE_1_0 (with the SR-11 fix as its load-bearing first step).

---

## SR-10 — Production-rule documentation

Audit basis:

W3-F03; W3-F02; W1-F12; W3-A9-B02; W3-A9-B03; W3-OPS-02; W3-OPS-E01

Relevant documentation locations (per rule):

[SOURCE] All five measured rules are documentation-only; each has a natural
existing placement site:

1. **Transactions rule** ("once any write to a collection is transactional,
   every write must use the raw session hatch" — W3-F03): README raw-access
   section (`README.md:913-930`, which already lists "sessions" as a raw
   concern) and the `get_collection`/`get_document_collection` rustdoc
   (`oximod_core/src/feature/model.rs:437-460`, `oximod/src/lib.rs:270-274`).
   Source context: no OxiMod call accepts a `ClientSession`
   (`model.rs`/`builder.rs` signatures) — the gap is structural and the rule
   is the accurate user-facing consequence. Sessions on the typed surface
   remain frozen post-1.0 (P-2); only the rule is published.
2. **Dotted paths, not whole models** (W3-A9-B02/B03, MongoDB `$set`/
   `replaceOne` semantics, raw-controlled, deliberately no finding ID):
   `UpdateExpression` rustdoc (`oximod/src/lib.rs:760-780`), README updates
   section, and `update_by_id` docs.
3. **`#[serde(alias)]` is not a rename migration** (W3-F02): the serde-rename
   paragraph (`oximod/src/lib.rs:130-131`, README serde-rename coverage) —
   note the typed DSL exposes no alias-spelling accessor, so typed queries
   silently miss legacy-keyed documents during a rollout.
4. **Never bare `#[serde(default)]` on an evolving field** (W1-F12): the
   Construction-and-defaults sections (`oximod/src/lib.rs:133-168`,
   README defaults section) — state that `#[default(...)]` is
   construction-time only (already documented) and that read-side defaults
   need `#[serde(default = "path")]`, with the zero-value write-back trap
   named.
5. **`init_global()` once; don't discard the `Result`** (W3-OPS-02/E01):
   `OxiClient::init_global` rustdoc
   (`oximod_core/src/feature/conn/client.rs:109-115` already documents the
   refusal; add the discarded-`Result` warning) and the README client table
   (`README.md:893`).

Current coverage:

Rules 4's construction-time scope and 5's once-semantics are partially
documented; the operational warnings (traps) are absent everywhere; rules
1-3 are entirely absent.

Missing placement/guidance:

As enumerated per-rule above.

Smallest sound resolution:

[RECOMMENDATION] Publish the five rules verbatim-in-substance at the sites
listed, each one to three sentences. No code change; none of the five
behaviours is chargeable to OxiMod (all raw-controlled or safe-by-design per
the audit).

Resolution class:

documentation

Compatibility impact:

None.

Estimated scope:

SMALL

Verification:

Documentation/example review; any code snippets added must compile
(SR-11 discipline).

Confidence:

HIGH

Open maintainer questions:

Whether rule 1 belongs in the README limitations list as well
(`README.md:1220-1262` — recommended yes, one line).

Recommended maintainer decision:

DOCUMENT_PRE_1_0

---

## SR-11 — Published examples with undeclared fields

Audit basis:

W1-F14; W1-B-01

Affected documentation/examples:

[MEASURED→SOURCE] The audit's three flagged docs.rs crate-root blocks map to
`oximod/src/lib.rs` doc comments, all fenced `rust,ignore`:

1. `oximod/src/lib.rs:290-300` — filters/logical-expressions fragment;
   references `user.role`, undeclared on any `User` on the page.
2. `oximod/src/lib.rs:314-325` — arrays/embedded fragment; references
   `user.tags`, `user.scores`, `user.addresses`, all undeclared. (This is
   the block whose non-compilability blinded the audit to
   `elem_match_nested` — see SR-9.)
3. `oximod/src/lib.rs:333-341` — pagination fragment; references `user.role`.

[SOURCE] The same transcription records blocks 9-11
(`lib.rs:353-362` `Article`, `lib.rs:369-379` `Place`, `lib.rs:389-398`
`login_count`/`nickname`) as equally non-self-contained but beyond the
audit's 8-block cap; W1-F14's count of three is the audit's bounded claim
and is not restated upward here. The `UpdateExpression` re-export doc
(`lib.rs:770-779`) repeats block 11's fragment.

Source-derived confirmation:

Confirmed by direct read; the blocks are `ignore`-fenced so no doctest ever
compiled them — the recurrence mechanism as well as the defect.

Smallest sound resolution:

[RECOMMENDATION] Declare the missing fields. Cleanest form: give the
"Typed queries" section one worked example model (declaring `role`, `tags`,
`scores`, `addresses`, `login_count`, `nickname` alongside the existing
fields) that the subsequent fragments visibly reference. Recurrence
protection (optional but cheap): convert the fragments from `ignore` to
compiled `no_run` doctests with hidden scaffolding, so `cargo test --doc`
compile-checks them permanently; same for blocks 9-11 while in the file.

Resolution class:

documentation

Compatibility impact:

None (doc comments only; no API or behaviour change).

Estimated scope:

SMALL (field declarations); SMALL-MEDIUM if fragments are converted to
compiled doctests (scaffolding for async fragments).

Verification:

`cargo test --doc -p oximod` after conversion; manual review otherwise;
README spot-check for the same fragments (README's corresponding examples
declare their fields — verified for the arrays section at
`README.md:576-596`).

Confidence:

HIGH

Open maintainer questions:

Convert to compiled doctests (recommended) or minimal field declarations
only?

Recommended maintainer decision:

DOCUMENT_PRE_1_0 (with compiled-doctest conversion as the recommended form).

---

## SR-12 — Derive attribute diagnostics

Audit basis:

W1-F10; W0-F02; W0-F03; F-V02; W1-M-X01

Audit observation being explained:

[MEASURED] A struct-level `///` (i.e. `#[doc]`), `#[allow(dead_code)]`, and
`#[non_exhaustive]` are rejected by `#[derive(Model)]`; the diagnostic's
span points at the line but its text never names the attribute; in a using
project every rejection cascades into a misleading E0599 (`new` not found,
14 irrelevant candidates — W0-F03) or cross-struct E0277/E0119 chains; a
mistyped `_id` type yields a generic E0308 never naming `_id` (W0-F02).
Field-level `///` is fine. (`#[deny]`/`#[warn]` remain single-observer per
the audit's own narrowing and are treated here as the same mechanism class.)

Source locations:

- **Rejection mechanism** [SOURCE]:
  `oximod_macros/src/helpers/model_attrs.rs:66-143` — the struct-level
  attribute loop allowlists exactly `model`, `serde`, `collection`, `db`,
  `index_max_retries`, `index_max_init_seconds`,
  `document_id_setter_ident`, `hooks`; **everything else** — including
  `#[doc]` (what `///` desugars to), `#[allow]`, `#[non_exhaustive]`,
  `#[cfg_attr]` products — hits the fallthrough
  `"Unsupported attribute for #[derive(Model)]"` (lines 105-109 embedded,
  138-143 collection), which never interpolates the attribute name.
- **Field-level asymmetry** [SOURCE]:
  `oximod_macros/src/helpers/model_fields.rs:114-154` — field attributes are
  matched with `if/else if` for `index`/`validate`/`default` and everything
  else is silently ignored; hence field-level `///` works. The struct level
  rejects-by-default; the field level ignores-by-default.
- **Cascade mechanism** [SOURCE]: `oximod_macros/src/lib.rs:104-108` —
  `expand_model(&input).unwrap_or_else(|error| error)`: on any attr error
  the derive's entire output is the `compile_error!` tokens, so no `new()`,
  no `Model`/`Queryable`/`FieldSchema` impls exist, producing the E0599
  (`new` not found; the 14 candidates are unrelated traits with `new` in
  scope) and, for a failed embedded model, cross-struct E0277
  (`FieldSchema` unsatisfied) blaming the *containing* struct's derive —
  exactly F-V02's shapes.
- **W0-F02 `_id` E0308** [SOURCE]:
  `oximod_macros/src/default/id_setter.rs:32-41` — the generated id setter
  hard-codes `self._id = Some(id)` with `id: ObjectId`; a non-`Option<ObjectId>`
  `_id` therefore fails type-checking *inside generated code*, yielding a
  generic E0308 that names neither `_id` nor the requirement.

Existing UI coverage:

`oximod/tests/ui/` covers query-surface rejections only; no case covers
struct-attribute handling or `_id` typing. (Trybuild run passes unchanged —
no baseline constrains this area.)

Reproduction/explanation:

Fully explainable from source; deterministic.

Smallest sound resolution:

[RECOMMENDATION] Three graduated parts:

1. **Stop rejecting inert standard attributes (core fix, SMALL):** in
   `collect_model_attrs`, ignore attributes that are not the derive's
   registered helpers — at minimum skip `doc`, `allow`, `warn`, `deny`,
   `expect`, `cfg`, `cfg_attr`, `non_exhaustive`, `derive`, `repr`,
   `automatically_derived` (rustc itself validates these). This makes
   struct-level `///`, `#[allow(dead_code)]`, and `#[non_exhaustive]`
   compile, eliminating the highest-frequency encounters and their cascades
   outright, and aligning struct-level policy with the existing field-level
   policy.
2. **Name the attribute in the remaining rejection (TRIVIAL):** interpolate
   the path into the message for whatever still lands in the fallthrough.
3. **Targeted `_id` diagnostic (SMALL-MEDIUM, optional):** detect a
   collection-model `_id` whose declared type is not `Option<ObjectId>`
   during field collection and emit a spanned `compile_error!` naming `_id`
   and the required type, pre-empting the generated-code E0308. (Cascade
   mitigation beyond parts 1-3 — emitting best-effort impls alongside
   errors — is MEDIUM and judged not necessary once part 1 removes the
   common triggers.)

Resolution class:

code

Public API impact:

None (compile surface only).

Semantic/backwards-compatibility impact:

**Widening, not breaking**: code that previously failed to compile now
compiles (struct docs, lint attributes, `non_exhaustive`). Note one
consequence the maintainer should own deliberately: `#[non_exhaustive]` +
`derive(Model)` becomes a supported combination (the derive generates
`new()`/`Default`, so it remains constructible; semver implications of
non_exhaustive models are the consumer's concern). No runtime, BSON,
validation, index, or error-matching change.

Dependency/version impact:

None.

Estimated scope:

Parts 1+2 SMALL; part 3 SMALL-MEDIUM.

Proposed `oximod/tests/ui` regression:

New pass-cases compiled as ordinary tests (struct-level `///`,
`#[allow(dead_code)]`, `#[non_exhaustive]` on both model kinds — e.g. a
`compile_pass` module in an existing test file) plus UI compile-fail
baselines for the *named* rejection message and, if part 3 ships, the
targeted `_id` diagnostic. Existing 11 UI baselines must pass unchanged.

External re-verification:

F-V02 probe matrix P00-P08; W0-C08 (`_id` and cascade shapes); W1-M-X01
attribute matrix (its single-observer rows would be settled by the new
pass-cases).

Nearby regression risks:

`#[serde]` container handling (already allowlisted — must stay);
`ModelKind::from_attrs` (`helpers/model_kind.rs:35-68`) iterates only
`model` attrs and is unaffected; macro unit test
`unsupported_model_attributes_are_rejected` (`model_attrs.rs:316-338`) pins
the current reject-all behaviour for `#[unknown]` and will need its
expectation reviewed — flagged per test discipline (it should keep passing
if part 1 skips only the inert-standard list rather than ignoring
everything; this diagnosis recommends the explicit skip-list precisely to
keep that test valid).

Confidence:

HIGH

Open maintainer questions:

1. Skip-list (keeps rejecting genuinely unknown attrs, preserves the
   existing unit test) vs ignore-all-unregistered (field-level parity,
   loosest)? This diagnosis recommends the skip-list.
2. Ship part 3 (`_id` diagnostic) pre-1.0?

Recommended maintainer decision:

FIX_PRE_1_0 (parts 1+2; part 3 at maintainer's option).

---

## SR-13 — page() behaviour over undeserializable documents

Audit basis:

W3-A9-B05 capability evidence; no finding ID by design

Audit observation being explained:

[MEASURED] With 5 poison rows among 45 documents: `query().all()` → one
`Err`, 0/45 processed. Paginated idiom (page size 10): pages 1-4, each
containing poison, each returned
`Err (Serialization error: Failed to deserialize typed query result)`,
losing the 35 valid documents that shared those pages and naming none of the
poison `_id`s; page 5 (clean) returned `Ok(len=5)`; `migrated=5, lost=35`
(`evidence/W3-A9-B05/stdout.txt` I2 rows; `isolation-idiom-table.csv`).

Source locations:

- `oximod_core/src/query/builder.rs:385-408` — `.page()` is a **modifier**
  (computes skip/limit); it is not an execution terminal.
- `oximod_core/src/query/builder.rs:629-674` — `.all()` is the single read
  terminal for paginated queries; the cursor loop maps every
  `deserialize_current` failure to
  `Err(OxiModError::Serialization { .. })` and propagates it immediately
  (line 666-671). There is **no silent-drop path**: any window containing an
  undeserializable document returns `Err`, and windows without one return
  `Ok`.

Source-derived mechanism and reconciliation:

[SOURCE] `.page(p, n).all()` and `.all()` execute identical code; the only
difference is the skip/limit window. The measured page-level results are
exactly this mechanism: loud `Err` per poisoned window, `Ok` for the clean
window. [INFERENCE] The SR-13 ledger recommendation "make `.page()` fail as
loudly as `.all()`" is therefore **already true mechanically at the audited
baseline**, and the gaps-report characterisation of the bounded terminal as
"silent" is not supported by the case's own primary evidence (every poisoned
page returned `Err`; nothing returned `Ok` while dropping documents). The
audit's *numbers* (`lost=35`, `poison_ids_reported=0`) all stand; "lost"
counts valid documents rendered unreachable through that idiom because their
window's terminal errs wholesale, and "not named" is real: the propagated
error carries no document `_id` (driver `BsonDeserialization` has no
document context and OxiMod adds none — the W1-F13 residue). This
explanation belongs to the remediation record and does not modify the frozen
audit's text or its capability-matrix row.

Existing internal coverage:

`oximod/tests/query_execution.rs`/`find.rs` cover `.page()` windows on
well-formed data; no internal test covers a poisoned window (no test pins
the `Err`-per-window contract).

Reproduction/explanation:

Fully explainable from source; the evidence's per-page `Err` lines are the
predicted output of `builder.rs:666-671`.

Smallest sound resolution:

[RECOMMENDATION]

1. **Documentation (the actual gap):** at `.page()`/`.all()` docs and the
   README pagination section, state: a page window containing an
   undeserializable document fails as a whole (`Err`), the valid documents
   sharing that window are not retrievable through the typed terminal, and
   the error does not identify the offending document; route
   diagnosis/repair to `get_document_collection()` + `bson::from_document`
   (the audit's `lost=0` idiom, already the documented raw hatch).
2. **Optional small code improvement (shared with the W1-F13 family):** in
   `.all()`'s deserialize-error arm, read the current raw document's `_id`
   (`cursor.current()` is available before deserialization) and include it
   in the error message — turning every poisoned-window failure, paginated
   or not, into an actionable signal. Changes Display text only; variant
   unchanged.

The premise "make `.page()` as loud as `.all()`" needs no code: they are the
same terminal. That specific asymmetry claim is recorded as **unsupported by
evidence** at the source level.

Resolution class:

documentation (+ optional error-context code); the loud/silent-asymmetry
premise itself: unsupported by evidence

Public API impact:

None. (Optional part 2 changes error message text.)

Semantic/backwards-compatibility impact:

Doc: none. Part 2: `Display` string of typed-query deserialization errors
gains the `_id` — string-matching consumers could notice; variant and
source-chain unchanged. No BSON, validation, or index change.

Dependency/version impact:

None.

Estimated scope:

Doc SMALL; part 2 SMALL.

Proposed internal regression:

New `oximod/tests/query_execution.rs` case (or sibling file): raw-insert one
malformed document among valid ones; assert `.all()` → `Err`, a clean
`.page()` window → `Ok`, a poisoned window → `Err`; if part 2 ships, assert
the message names the `_id`.

External re-verification:

W3-A9-B05 idioms I1 and I2 (poison-document pagination case).

Nearby regression risks:

`first()`'s deserialization error shape (`builder.rs:552-567`, currently
`Database` — see SR-2's class divergence); cursor-advance error path;
`delete_one`/`update_one` post-image deserialization (W2-F04 territory —
explicitly not in SR scope).

Confidence:

HIGH

Open maintainer questions:

1. Confirm acceptance that no loud-vs-silent code change is warranted (the
   external re-verification should then target the documented contract, not
   a behaviour change).
2. Ship the `_id`-enrichment (recommended; it also discharges the "named
   none of the poison rows" pain measured across I2/I4)?

Recommended maintainer decision:

DOCUMENT_PRE_1_0, plus the optional `_id` error-context enrichment as a
small approved code change if desired.

---

## Cross-item interactions

- **SR-9 ↔ SR-11:** the non-self-contained crate-root block 7 is both an
  SR-11 defect and the direct cause of the audit's inability to discover
  `elem_match_nested` (SR-9). Fixing SR-11 is load-bearing for SR-9's
  external re-verification.
- **SR-3 ↔ SR-7:** a public `init_indexes()` (SR-3) also converts SR-7's
  first-save failure into a deterministic startup failure; SR-7's
  compile-time check removes the single-model conflict class before either
  runtime path can see it.
- **SR-2 ↔ SR-13:** both touch generated error construction; the `_id`
  enrichment (SR-13 part 2) and any variant remap (SR-2 lane B) should land
  as one reviewed change to the error surface if both are approved.
- **SR-1 ↔ SR-10:** if the hook remedy is documented for SR-1, W2-F06's
  both-hooks caveat must be documented with it (audit requirement), and the
  hooks doc section is also SR-10-adjacent territory.
- **SR-4 ↔ SR-5:** same placement sites (index routing sentences); one
  documentation change.
- **SR-6 ↔ SR-13:** both stem from typed-collection deserialization on
  paths users expect to be data-agnostic; their regression tests can share
  the malformed-document seeding helper.

## Proposed implementation batches

DIAGNOSIS ONLY — proposals may be recorded here, but no batch is authorised.

- **Batch D (documentation only; lowest risk; no approval-gated semantics):**
  SR-4, SR-5, SR-10, SR-11, SR-1 lane 1, SR-2 lane A, SR-9 (doc), SR-13
  (doc), SR-3 part 1. One coherent docs pass across
  `oximod/src/lib.rs`, `README.md`, and the error/trait rustdoc.
- **Batch C1 (small code, narrow semantics — each needs its listed
  approval):** SR-6 (exists via document collection), SR-12 parts 1+2,
  SR-7 parts 1+2, SR-13 part 2 (`_id` enrichment), SR-2 lane B one-line
  insert remap.
- **Batch C2 (additive public API — explicit approval required):** SR-3
  `init_indexes()`/`init_indexes_from()`; SR-8 `Option<T>` ordered+numeric
  operators; optionally SR-1 `#[validate(nested)]`; optionally SR-12 part 3.
- Explicitly NOT proposed in any batch: automatic validation descent
  (SR-1), full error classifier as default (SR-2), generated
  `From<Embedded> for Bson` (SR-9), `#[index(partial_filter)]` (SR-5), any
  P-1..P-12 surface.

## Proposed breaking or semantic changes requiring maintainer approval

1. **SR-2 lane B:** insert-path errors remapped `Connection` → `Database`.
   Existing 0.3.0 `match` arms on `OxiModError::Connection` around `save()`
   change behaviour. No compile breakage. Persisted BSON: no. Validation
   timing: no. Index lifecycle: no. **Error matching: yes.** Migration
   guidance: yes (one changelog section with the before/after mapping
   table).
2. **SR-6:** `exists()` over a matched undeserializable document changes
   `Err` → `Ok(true)`. No compile breakage. **Error matching: yes,
   narrowly.** Migration guidance: one changelog sentence.
3. **SR-7 part 1:** two-text-index / duplicate-name declarations stop
   compiling. **Compile breakage: yes**, limited to declarations that today
   deterministically fail every save (plus the never-saved-model edge
   case). BSON/validation/index lifecycle for valid declarations: no
   change. Migration guidance: the compile error text itself.
4. **SR-7 part 2 and SR-13 part 2:** `Display` text of the index error and
   the typed-query deserialization error changes (collection name; document
   `_id`). Variants unchanged; string-matching consumers only.
5. **SR-12 part 1:** previously-rejected struct attributes (`///`,
   `#[allow]`, `#[non_exhaustive]`) begin compiling — a widening change;
   `#[non_exhaustive]` + `Model` becomes a supported combination.
6. **SR-3 part 2 / SR-8 / SR-1 lane 2:** additive public API (generated
   inherent methods, new `Field<Option<T>>` methods, new attribute key).
   Additive, but generated inherent names can collide with user-defined
   inherent methods of the same name (SR-3's `init_indexes`).

No proposed change alters persisted BSON, validation timing for existing
code, or the index lifecycle of currently-valid declarations.

## Post-1.0 items encountered during diagnosis

Record only interactions. Do not promote or implement.

- **P-2 (sessions/transactions):** SR-10 rule 1 documents the production
  rule only; the typed session surface stays frozen.
- **P-6 (poison-tolerant enumeration terminal):** SR-13's diagnosis
  documents the existing wholesale-failure contract; a tolerant terminal
  remains frozen. The `_id` enrichment is error-context, not a new
  terminal.
- **P-7 (typed operators on bare `u64`/`ObjectId`/enums):** SR-8's
  `Option<T>` fix pattern (marker-trait impls / inherent methods) would
  mechanically extend to parts of P-7; **no such extension is proposed** —
  the `u64` half is not chargeable (BSON has no u64) and the rest is
  frozen.
- **P-8 (compound text index):** SR-7's compile check must not accidentally
  foreclose future compound-text work; the check as specified only rejects
  *conflicting* declarations, which the server rejects anyway.
- **P-12 (clear-field affordance):** adjacent to SR-8's `Option` surface
  (`unset()` exists only on `Option<T>` — coherence confirmed by F-V01);
  nothing proposed.

## D0 final recommendation summary

SR-1:

DOCUMENT_PRE_1_0 (boundary + both remedies + W2-F06 caveat); optional
upgrade to FIX_AND_DOCUMENT_PRE_1_0 with an opt-in `#[validate(nested)]`.

SR-2:

FIX_AND_DOCUMENT_PRE_1_0 — variant-doc correction (mandatory core) plus the
one-line insert remap if the maintainer accepts the error-matching change;
doc-only is the fallback that still discharges the finding.

SR-3:

FIX_AND_DOCUMENT_PRE_1_0 — trigger-sentence placement at the `#[index]`
docs plus additive `init_indexes()`/`init_indexes_from()` (public API
approval required).

SR-4:

DOCUMENT_PRE_1_0 — label the derived-composite-key substitute unsafe next
to every compound-index routing sentence.

SR-5:

DOCUMENT_PRE_1_0 — add the partial/filtered-index routing sentence;
capability itself remains an intentional boundary.

SR-6:

FIX_PRE_1_0 — probe existence through the document collection (one line);
narrow error-behaviour change flagged.

SR-7:

FIX_PRE_1_0 — compile-time cross-field check (two text-implying /
duplicate-name) plus collection-naming in the runtime index error;
compile-surface change flagged.

SR-8:

FIX_PRE_1_0 — additive ordered+numeric operators on `Field<Option<T>>`
taking inner-type arguments; documentation lane as fallback.

SR-9:

DOCUMENT_PRE_1_0 — fix the block-7 example (with SR-11), document the
`From<T> for Bson` remedy and the `elem_match_nested`/`positional` pairing;
`elem_match_nested` already exists and is internally tested.

SR-10:

DOCUMENT_PRE_1_0 — publish the five measured production rules at the
enumerated placement sites.

SR-11:

DOCUMENT_PRE_1_0 — declare the missing fields; convert fragments to
compiled doctests for recurrence protection.

SR-12:

FIX_PRE_1_0 — skip inert standard attributes, name the attribute in
remaining rejections; optional targeted `_id` diagnostic.

SR-13:

DOCUMENT_PRE_1_0 — document the wholesale-window failure contract at
`.page()`/`.all()`; optional `_id` error enrichment; the loud-vs-silent
asymmetry premise is unsupported by evidence (source and the case's own
stdout agree the paginated path fails loudly per window).

## D0 completion declaration

Implementation started:

NO

Source implementation files modified:

NO

Audit evidence modified:

NO

Post-1.0 feature work performed:

NO

Ready for maintainer disposition review:

YES
