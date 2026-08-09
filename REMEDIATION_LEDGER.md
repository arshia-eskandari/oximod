# OxiMod Audit Remediation Ledger

## Status

CONTROLLED — DIAGNOSIS NOT YET STARTED

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
| SR-1 | Give the embedded `#[validate]` gap a signal, or document it with its remedy | W2-F05; W2-F06; W2-V06; W2-V07; W2-A4-E02 | CODE_OR_DOC | PENDING_DIAGNOSIS | UNDECIDED | W2-A4-E02 / W2-V07 validation matrix; hook-remedy coverage if relevant |
| SR-2 | Correct the error-variant documentation or the variant meanings | W1-F11; W1-V07 | DOC_OR_CODE | PENDING_DIAGNOSIS | UNDECIDED | W1-V07 error-classification cases |
| SR-3 | State the index-establishment trigger and provide a write-independent establish/verify path | W3-F01; W3-V01; W3-A6-E01; W3-A8-B02; W3-A9-X02 | CODE_AND_DOC_CANDIDATE | PENDING_DIAGNOSIS | UNDECIDED | W3-V01 index-establishment cluster |
| SR-4 | Warn that a derived composite key is not a safe substitute for compound uniqueness | W1-F16; W1-B-07; W2-A3-B02; W2-A4-X03; W2-A5-X02 | DOC_CANDIDATE | PENDING_DIAGNOSIS | UNDECIDED | Documentation verification; no new capability implied |
| SR-5 | Make the partial/filtered-uniqueness raw-driver boundary explicit | W1-F16 family; W2-A3-B02; W2-A4-X03; W2-A5-X02 | DOC_CANDIDATE | PENDING_DIAGNOSIS | UNDECIDED | Documentation verification; raw hatch remains boundary unless separately approved |
| SR-6 | Close the `exists()` / `count()` inconsistency, or document it | W2-F02; W2-V03; W2-V04 | CODE_OR_DOC | PENDING_DIAGNOSIS | UNDECIDED | W2-V03 and W2-V04 malformed-document probes |
| SR-7 | Give conflicting text-index / duplicate-index-name declarations an earlier signal | W2-F07; W2-V08; W2-A4-E01; W2-A3-B02 | CODE_OR_DIAGNOSTIC | PENDING_DIAGNOSIS | UNDECIDED | W2-A4-E01 / W2-V08 declaration matrix |
| SR-8 | Expose ordered/numeric operators on `Option<T>`, or document the supported boundary/workaround | W2-F01; W2-V01; W2-A7-B02; W2-A3-B01 | CODE_OR_DOC | PENDING_DIAGNOSIS | UNDECIDED | W2-V01 operator compile matrix |
| SR-9 | Make `elem_match` on `Vec<Embedded>` reachable and document the array-operator remedy | W2-F03; W2-V02; W2-A4-B03 | CODE_AND_OR_DOC | PENDING_DIAGNOSIS | UNDECIDED | W2-V02 array/operator matrix |
| SR-10 | Publish the measured production rules | W3-F03; W3-F02; W1-F12; W3-A9-B02; W3-A9-B03; W3-OPS-02; W3-OPS-E01 | DOC_CANDIDATE | PENDING_DIAGNOSIS | UNDECIDED | Documentation/example verification |
| SR-11 | Fix the three published code blocks that reference undeclared fields | W1-F14; W1-B-01 | DOC_CANDIDATE | PENDING_DIAGNOSIS | UNDECIDED | Compile/check affected documentation examples |
| SR-12 | Improve derive attribute diagnostics | W1-F10; W0-F02; W0-F03; F-V02; W1-M-X01 | CODE_OR_DIAGNOSTIC | PENDING_DIAGNOSIS | UNDECIDED | F-V02 compile matrix plus relevant `oximod/tests/ui` coverage |
| SR-13 | Make `.page()` fail as loudly as `.all()` over undeserializable documents, or document the difference | W3-A9-B05 capability evidence; no filed finding ID by design | CODE_OR_DOC | PENDING_DIAGNOSIS | UNDECIDED | W3-A9-B05 poison-document pagination case |

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

