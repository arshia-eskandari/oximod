# OxiMod Audit Remediation Ledger

## Status

D6 IMPLEMENTATION COMPLETE — SR-2 READY_FOR_REVERIFY

## Baseline

Audited release:

OxiMod 0.3.0

Audited source commit:

51cdf57dce8b7a9615008a2e325e11894e64cd39

Remediation branch:

audit-remediation

Remediation worktree:

/home/arshia/Code/audit-remediation

Frozen audit record:

/home/arshia/Code/oximod-blackbox-audit-final

Closed audit archive:

/home/arshia/Code/oximod-blackbox-audit-closed-2026-08-08.tar.gz

The three crates.io 0.3.0 packages — oximod, oximod_core, and
oximod_macros — all record the baseline Git SHA above in
`.cargo_vcs_info.json`.

## Authority

The completed black-box audit is immutable evidence.

Primary remediation inputs:

1. reports/final-report.md
2. reports/oximod-1.0-gaps.md
3. reports/final-claim-review.md
4. reports/finding-index.csv
5. reports/capability-matrix.csv
6. reports/coverage-matrix.csv
7. reports/claim-evidence-matrix.csv
8. run-manifests/final.json
9. assessments/final-fit.md

`reports/finding-index.csv` remains the audit authority for finding IDs,
classifications, severities, confidence, and verification counts.

This remediation ledger must not rewrite audit history. Source-aware diagnosis
may explain an observed behavior, but it does not retroactively alter the
black-box evidence.

## Audit conclusion that governs scope

The final audit reported:

- 31 findings.
- No Critical finding.
- No unambiguous 1.0 release blocker under the audit contract.
- 13 strongly recommended pre-1.0 remediation items, SR-1 through SR-13.
- 12 post-1.0 expansion opportunities, P-1 through P-12.
- Clean raw-driver escape hatches across all nine tested application archetypes.

Therefore:

- not every finding requires a code change;
- not every capability gap should become an OxiMod feature;
- post-1.0 opportunities must not silently enter the pre-1.0 remediation scope;
- the smallest technically sound resolution is preferred.

## State model

Each SR item has two independent controls.

Diagnosis state:

- PENDING_DIAGNOSIS
- DIAGNOSED
- MAINTAINER_DECIDED
- IMPLEMENTING
- READY_FOR_REVERIFY
- CLOSED

Maintainer decision:

- UNDECIDED
- FIX_PRE_1_0
- DOCUMENT_PRE_1_0
- FIX_AND_DOCUMENT_PRE_1_0
- INTENTIONAL_BOUNDARY
- DEFER_POST_1_0
- REJECT_WITH_EVIDENCE

Only the maintainer may move an item out of `UNDECIDED`.

A source-aware agent may recommend a decision but may not assign one.

## Closure rule

An item is not CLOSED merely because code compiles or an agent says it is fixed.

For a code-affecting item, closure requires:

audit evidence
→ source-aware diagnosis
→ maintainer decision
→ regression test
→ implementation
→ repository verification
→ source-hidden external re-verification
→ maintainer closure

Documentation-only items require:

audit evidence
→ source-aware diagnosis
→ maintainer decision
→ documentation change
→ documentation/example verification
→ maintainer closure

## Pre-1.0 remediation ledger

| ID | Audit recommendation | Audit basis | Initial resolution lane | Diagnosis | Decision | External re-verification target |
|---|---|---|---|---|---|---|
| SR-1 | Give the embedded `#[validate]` gap a signal, or document it with its remedy | W2-F05; W2-F06; W2-V06; W2-V07; W2-A4-E02 | CODE_OR_DOC | CLOSED | DOCUMENT_PRE_1_0 | W2-A4-E02 / W2-V07 validation matrix; hook-remedy coverage if relevant |
| SR-2 | Correct the error-variant documentation or the variant meanings | W1-F11; W1-V07 | DOC_OR_CODE | READY_FOR_REVERIFY | FIX_AND_DOCUMENT_PRE_1_0 | W1-V07 error-classification cases |
| SR-3 | State the index-establishment trigger and provide a write-independent establish/verify path | W3-F01; W3-V01; W3-A6-E01; W3-A8-B02; W3-A9-X02 | CODE_AND_DOC_CANDIDATE | CLOSED | FIX_AND_DOCUMENT_PRE_1_0 | W3-V01 index-establishment cluster |
| SR-4 | Warn that a derived composite key is not a safe substitute for compound uniqueness | W1-F16; W1-B-07; W2-A3-B02; W2-A4-X03; W2-A5-X02 | DOC_CANDIDATE | CLOSED | DOCUMENT_PRE_1_0 | Documentation verification; no new capability implied |
| SR-5 | Make the partial/filtered-uniqueness raw-driver boundary explicit | W1-F16 family; W2-A3-B02; W2-A4-X03; W2-A5-X02 | DOC_CANDIDATE | CLOSED | DOCUMENT_PRE_1_0 | Documentation verification; raw hatch remains boundary unless separately approved |
| SR-6 | Close the `exists()` / `count()` inconsistency, or document it | W2-F02; W2-V03; W2-V04 | CODE_OR_DOC | CLOSED | FIX_PRE_1_0 | W2-V03 and W2-V04 malformed-document probes |
| SR-7 | Give conflicting text-index / duplicate-index-name declarations an earlier signal | W2-F07; W2-V08; W2-A4-E01; W2-A3-B02 | CODE_OR_DIAGNOSTIC | CLOSED | FIX_PRE_1_0 | W2-A4-E01 / W2-V08 declaration matrix |
| SR-8 | Expose ordered/numeric operators on `Option<T>`, or document the supported boundary/workaround | W2-F01; W2-V01; W2-A7-B02; W2-A3-B01 | CODE_OR_DOC | CLOSED | FIX_PRE_1_0 | W2-V01 operator compile matrix |
| SR-9 | Make `elem_match` on `Vec<Embedded>` reachable and document the array-operator remedy | W2-F03; W2-V02; W2-A4-B03 | CODE_AND_OR_DOC | CLOSED | DOCUMENT_PRE_1_0 | W2-V02 array/operator matrix |
| SR-10 | Publish the measured production rules | W3-F03; W3-F02; W1-F12; W3-A9-B02; W3-A9-B03; W3-OPS-02; W3-OPS-E01 | DOC_CANDIDATE | CLOSED | DOCUMENT_PRE_1_0 | Documentation/example verification |
| SR-11 | Fix the three published code blocks that reference undeclared fields | W1-F14; W1-B-01 | DOC_CANDIDATE | CLOSED | DOCUMENT_PRE_1_0 | Compile/check affected documentation examples |
| SR-12 | Improve derive attribute diagnostics | W1-F10; W0-F02; W0-F03; F-V02; W1-M-X01 | CODE_OR_DIAGNOSTIC | CLOSED | FIX_PRE_1_0 | F-V02 compile matrix plus relevant `oximod/tests/ui` coverage |
| SR-13 | Make `.page()` fail as loudly as `.all()` over undeserializable documents, or document the difference | W3-A9-B05 capability evidence; no filed finding ID by design | CODE_OR_DOC | CLOSED | DOCUMENT_PRE_1_0 | W3-A9-B05 poison-document pagination case |


## Maintainer disposition notes — 2026-08-09

These notes are authoritative for implementation scope. They refine the
decision column without altering the frozen audit or the D0 diagnosis.

### SR-1

Pre-1.0 resolution is documentation only.

Do not implement automatic validation descent or `#[validate(nested)]` in the
current remediation campaign. Document that embedded validation does not
recurse automatically, document the custom-validator remedy, and document the
hook remedy together with the requirement to cover both `pre_save` and
`pre_save_mut`.

### SR-2

The pre-1.0 goal is a coherent error contract, not merely a one-line
`Connection` -> `Database` remap.

Before implementation, design and obtain maintainer approval for a complete
mapping in which `OxiModError` variants have one consistent meaning across the
public API. The preferred direction is failure-class semantics, while
preserving the original driver error through `source()`.

SR-2 is intentionally excluded from D1 and will receive its own implementation
phase.

### SR-3

Approve an explicit write-independent index-initialization API.

Preferred names:

- `init_indexes()`
- `init_indexes_from(...)`

Do not implement continuous drift detection or automatic re-establishment.
Those would change the established once-per-process lifecycle and are outside
this remediation.

SR-3 is intentionally excluded from D1.

### SR-4

Documentation only. Explicitly warn that a derived/composite mirror field is
not a safe substitute for a real MongoDB compound unique index.

### SR-5

Documentation only. Partial/filtered indexes remain an intentional raw-driver
boundary pre-1.0.

### SR-6

Approve the code correction: `exists()` should perform a document-level
existence probe rather than deserialize the matched model.

The narrow `Err` -> `Ok(true)` behavior change for an undeserializable matching
document is accepted.

### SR-7

Approve compile-time rejection of exactly the two audit-established
single-model conflicts:

- more than one text-implying declared index;
- duplicate literal declared index names.

Also approve adding collection context to the runtime index-creation error.

Do not generalize this into broad MongoDB index compatibility analysis.

### SR-8

Approve a pre-1.0 code solution for `Option<T>` operators using inner-value
arguments rather than blanket marker-trait implementations that would permit
nonsensical values such as `inc(None)`.

Ordered and numeric families are approved. Integer/bitwise operators may be
included only if they are a straightforward application of the same safe
design; otherwise leave them documented for later review.

SR-8 is intentionally excluded from D1.

### SR-9

Documentation only.

`elem_match_nested` already exists at the audited baseline and must not be
reimplemented.

Do not generate `From<Embedded> for Bson` pre-1.0: it would conflict with
consumer implementations of the documented workaround and creates an
infallible-conversion design problem.

### SR-10

Documentation only: publish the five measured production rules identified by
D0.

### SR-11

Documentation correction plus recurrence protection.

Fix the broken examples and convert them to compiled doctests where practical
without making the examples materially harder to read.

### SR-12

Approve:

1. an explicit skip-list for ordinary/inert standard Rust attributes;
2. naming the offending attribute in remaining unsupported-attribute errors.

Do not switch to ignoring every unknown struct attribute.

A targeted `_id` diagnostic is desirable, but it may be implemented in D1 only
if the macro can do so without falsely rejecting valid 0.3.0 type aliases,
qualified paths, or other already-accepted equivalent spellings. If that
cannot be guaranteed from syntax alone, STOP that sub-part and report it for a
later design decision rather than narrowing accepted source.

### SR-13

No loud-versus-silent behavior correction is authorized. D0 established that
poisoned page windows already fail loudly through the same `.all()` terminal.

Document the actual whole-window failure behavior and raw-document repair
route.

Adding the offending document `_id` to deserialization error context is
favored, but it belongs with the later SR-2 error-surface phase rather than D1.


## Post-1.0 scope fence

The following audit opportunities are FROZEN_POST_1_0 during the initial
remediation campaign:

P-1  aggregation entry point
P-2  sessions and transactions on the typed surface
P-3  bulk writes
P-4  change-stream surface
P-5  projections, streaming, and bounded peak memory
P-6  poison-tolerant enumeration terminal
P-7  typed operators on bare u64 / ObjectId / plain enums
P-8  compound / multi-field text index
P-9  relevance score as application data
P-10 database-per-tenant isolation
P-11 nameable Expression for query composition
P-12 generated/supported clear-field affordance

An agent may mention interactions with these items during diagnosis.

An agent may NOT implement, promote, or pull any of them into the pre-1.0
campaign without explicit maintainer approval.

## Permanent-regression principle

The black-box audit recommends a set of permanent regression candidates.

Do not mechanically copy every audit project into the repository.

Prefer:

- focused unit/integration regressions for corrected OxiMod behavior;
- `oximod/tests/ui` for compile-surface and diagnostic contracts;
- external/source-hidden smoke coverage for expensive process-boundary,
  MongoDB-lifecycle, change-stream, or multi-process behavior.

The original black-box reproducers remain evidence and external re-verification
targets; they are not automatically the ideal internal test implementation.

## Change-control rule

No implementation starts because an item is present in this ledger.

The order is:

diagnose all SR-1 through SR-13
→ review diagnosis
→ maintainer decides each item
→ approve bounded implementation batches
→ implement
→ verify
→ externally re-test


## D2 closure note — 2026-08-09

D2 documentation implementation commit:

`c4d04ed974a1de14c00eaec9ad468e7a70abbea1`

Closed items:

- SR-1
- SR-4
- SR-5
- SR-9
- SR-10
- SR-11
- SR-13

Maintainer review confirmed that D2 remained documentation-only and introduced
no runtime/API behavior change.

Verification included:

- `cargo fmt --all -- --check`
- `cargo test --doc --workspace`
- `cargo test -p oximod_core -p oximod_macros`
- `cargo test -p oximod --test compile_fail`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `git diff --check`

After the maintainer-requested wording corrections, formatting, workspace
doctests, and diff-check were rerun successfully. Final doctest counts were:

- `oximod`: 27 passed, 0 failed, 0 ignored
- `oximod_core`: 3 passed, 0 failed, 73 ignored
- `oximod_macros`: 1 passed, 0 failed, 3 ignored

The ignored `oximod_core` / `oximod_macros` doctests are pre-existing.

## D3 closure note — 2026-08-09

D3 implementation commit:

`9ef97c4335b802c74ad592ca539f5fb6a9e4aefc`

READY_FOR_REVERIFY ledger commit:

`498233f`

Source-hidden external re-verification:

`SR-8 = READY_TO_CLOSE`

Maintainer provenance check confirmed no change to README, manifests,
or OxiMod source/test trees between the implementation commit and the
READY_FOR_REVERIFY commit (`SOURCE_DIFF_EXIT=0`).

External report:

`REMEDIATION_D3_REVERIFY.md`

External report SHA-256:

`7b27cda84857003e12115ae8b5d0ab9ee5555b7514e7c1584da796cd16ec5704`

Archived evidence:

`~/Code/oximod-d3-reverify-2026-08-09.tar.gz`

Archive SHA-256:

`0dd3e3410ece77ab4053307381a2b01c9918dc0ba42727d03ca97779743447a0`

SR-8 is CLOSED.

## D4 closure note — 2026-08-10

D4 implementation commit:

`e588274b8424fb63fb28b4c7fcbb01ef654cadc2`

READY_FOR_REVERIFY state-recording commit:

`956640ff959082bdb0f106f3a8a335db7aa911cf`

Source-hidden external re-verification:

`SR-3 = READY_TO_CLOSE`

The external verification occurred before the READY_FOR_REVERIFY
bookkeeping transition was committed. Maintainer provenance verification
confirmed no change to README, manifests, or OxiMod source/test trees
between the implementation commit and the later state-recording commit
(`SOURCE_DIFF_EXIT=0`).

External report:

`REMEDIATION_D4_REVERIFY.md`

External report SHA-256:

`6a96665122ae71e46460bd8dd60d8d7f034583ecc3692db608592853c5182fbf`

Archived evidence:

`~/Code/oximod-d4-reverify-2026-08-10.tar.gz`

Archive SHA-256:

`9a82aacfe15e6d0eec37c752f2149260291918d89010224f5e1e195c9574f3f0`

SR-3 is CLOSED.
