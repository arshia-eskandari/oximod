# Maintainer provenance note

This file preserves the source-hidden D1 external re-verification report.

The verifier was instructed to test D1 implementation commit `09d6b7a`
through the path dependency:

`/home/arshia/Code/audit-remediation/oximod`

At verification time the remediation worktree HEAD was `3cf258a`.
That later commit changed only remediation-ledger state. Before closure, the
maintainer verified with `git diff --exit-code 09d6b7a 3cf258a -- oximod
oximod_core oximod_macros` that the candidate source consumed by the verifier
was identical to the implementation source committed at `09d6b7a`.

The verifier did not inspect candidate implementation source and did not
modify either the remediation repository or the frozen audit.

---

# OxiMod D1 remediation — source-hidden external re-verification (SR-6, SR-7, SR-12)

Date: 2026-08-09
Verifier: external consumer-side differential verification (source-hidden).

## Identities under test

| Identity | Dependency | Resolved in consumer Cargo.lock |
|---|---|---|
| candidate (`cand/`) | `oximod = { path = "/home/arshia/Code/audit-remediation/oximod" }` (stated provenance: remediation commit 09d6b7a) | `oximod 0.3.0` path source (no registry line), with path `oximod_core` / `oximod_macros` |
| baseline (`base/`) | `oximod = "=0.3.0"` | `oximod 0.3.0` from `registry+https://github.com/rust-lang/crates.io-index` |

Toolchain: rustc/cargo 1.97.1. MongoDB: `mongodb://127.0.0.1:27019/?replicaSet=rs0`.
Frozen audit consulted READ-ONLY: `/home/arshia/Code/oximod-blackbox-audit-final`
(cases W2-V03, W2-V04, W2-V08, W2-A4-E01, F-V02 and their evidence directories).

## Source-hidden compliance

- No file under `/home/arshia/Code/audit-remediation` was read, grepped, listed,
  or otherwise inspected. Cargo compiled the path dependency normally.
- Candidate behavior was matched externally: the `Index` error variant via its
  `Debug` rendering, message text via `Display`, cause via
  `std::error::Error::source()` + `downcast_ref::<mongodb::error::Error>()`.
- The frozen audit was not modified; nothing was written outside
  `/tmp/oximod-d1-reverify`. No commits, no pushes.

## SR-6 — `exists()` must not require deserialization

Consumer: `{cand,base}/sr6` (adapted from frozen W2-V03 §B7/B7b, W2-V04 §C5/C6).
Model `Item { _id, sku: String, qty: i32 }`; raw-inserted matching document with
`qty` as a BSON string (malformed for the model). Fresh DB dropped per run.

| Probe | candidate | baseline 0.3.0 |
|---|---|---|
| typed read of malformed doc (`find_by_id`) | Err — `invalid type: string "three", expected i32` | Err (same) |
| `exists({"sku":"MATCH-1"})` | **Ok(true)** | **Err** — "Failed to check document existence" / BsonDeserialization |
| `count({"sku":"MATCH-1"})` | Ok(1) | Ok(1) |
| `exists == (count > 0)` | true | not evaluable (exists errored) |
| `exists({"sku":"NO-SUCH-SKU"})` | Ok(false) | Ok(false) |

The baseline failure reproduces the frozen audit's historical observation
(W2-V03 `B7_EXISTS_ALL Err` / `B7b_EXISTS_CORRUPT_ONLY is_err=true`).

**SR-6: PASS.**

## SR-7 — declaration-local index conflicts

Consumers: `{cand,base}/sr7probe` (compile matrix, one probe per `cargo check -p`)
and `{cand,base}/sr7runtime` (Case D).

| Probe | candidate | baseline 0.3.0 |
|---|---|---|
| A0: two explicit `#[index(text, name=…)]` | **FAIL exit 101** | compiles (exit 0) |
| A1: explicit text + `#[index(text, weight = 5, name=…)]` | **FAIL exit 101** | compiles |
| A2 (primary Case A): explicit text + weight-only text-implying `#[index(weight = 5, name=…)]` | **FAIL exit 101** | compiles |
| B1 (Case B): duplicate literal `name = "pb1_dup_named_idx"` on two indexes | **FAIL exit 101** | compiles |
| C1 (Case C, positive control): 4 indexes (unique/text/order/sparse), distinct names, one text | compiles, 0 warnings | compiles |
| D0 (Case D compile control): two models, same collection, same index name, different keys | compiles, 0 warnings | compiles |

Candidate diagnostics (verbatim, from `logs/cand-sr7-*.stderr.txt`):

- A0/A1/A2: ``error: conflicting #[index] declarations: field `summary` declares a text or text-implying index, but field `title` already declares one; MongoDB allows at most one text index per collection`` — span on the conflicting attribute. Clearly identifies the conflicting text/text-implying declaration.
- B1: ``error: duplicate #[index] name `pb1_dup_named_idx`: an index with this name is already declared on field `field_a` `` — clearly identifies the duplicate name.

Case D runtime (candidate `logs/cand-sr7d-run.stdout.txt`):
- compilation succeeded; ModelOne `save()` Ok, index `shared_conflict_idx` created;
- ModelTwo `save()` Err; Debug variant `Index { … }` (variant_is_index=true);
- Display: ``Index error: Failed to create indexes for collection `shared_docs` ``
  — **contains the collection name** (baseline Display lacks it:
  `Index error: Failed to create indexes for collection`);
- `source()` available; downcast to `mongodb::error::Error` yields server code 86
  `IndexKeySpecsConflict`;
- remains a runtime issue (no cross-model compile rejection), matching the
  "no cross-model static analysis required" constraint.

**SR-7: PASS** (both poisoned declarations fail early with specific diagnostics;
legitimate multi-index model still compiles; cross-model case stays runtime with
collection-named error context and reachable source).

## SR-12 — struct-attribute acceptance / diagnostics / `_id` compatibility

Consumer: `{cand,base}/sr12probe`; probes p00/p01/p03/p04/p05 copied from frozen
F-V02 (db literal renamed only), p09–p12 new.

| Probe | candidate | baseline 0.3.0 |
|---|---|---|
| p00 positive control | 0 | 0 |
| p01 `///` on collection model (above derive) | **0** | 101 |
| p03 `///` on `#[model(embedded)]` struct | **0** | 101 |
| p04 `#[allow(dead_code)]` | **0** | 101 |
| p05 `#[non_exhaustive]` | **0** | 101 |
| p12 combined `///` + allow + non_exhaustive on collection + `///` on embedded | **0** | 101 |
| p09 unregistered `#[definitely_not_a_real_attribute]` | **101** | 101 |
| p10 `_id: MaybeOid` (type alias for `Option<ObjectId>`) | 0 | 0 |
| p11 `_id: std::option::Option<ObjectId>` | 0 | 0 |

p09 diagnostics:
- candidate: ``error: Unsupported attribute `#[definitely_not_a_real_attribute]` for #[derive(Model)]`` — **names the attribute**; rustc's own `cannot find attribute … in this scope` also appears (acceptable per spec).
- baseline: `error: Unsupported attribute for #[derive(Model)]` — attribute not named (the W1-F10 defect).

`_id` probes: both spellings accepted by baseline remain accepted by the
candidate — no narrowing. (The targeted `_id` diagnostic was deferred by D1 and
was not required here.)

**SR-12: PASS.**

## Discrepancies

None against the frozen audit's recorded baseline behavior, and none against
the intended D1 behavior for these three items. All baseline observations made
here reproduce the audit's records (0.3.0 accepts both poisoned index
declarations; rejects doc/allow/non_exhaustive; `exists()` errors on the
malformed matching document; Index error Display lacks the collection name).

## Recommendation (maintainer to confirm; nothing marked closed here)

- SR-6: READY_TO_CLOSE
- SR-7: READY_TO_CLOSE
- SR-12: READY_TO_CLOSE

## Artifact map

- `run.sh`, `run-probes.sh` — evidence-preserving runners (real exit codes, no pipelines).
- `probes/sr7/*.rs`, `probes/sr12/*.rs` — shared compile probes.
- `cand/`, `base/` — consumer workspaces (members `sr6`, `sr7probe`, `sr7runtime`, `sr12probe`), each with own `Cargo.lock` and `target/`.
- `logs/commands.log` — every measured command with cwd and true exit status.
- `logs/<label>.{stdout,stderr}.txt`, `logs/<label>.exit` — per-command evidence.

## Preserved evidence archive

Complete external re-verification artifacts were archived outside the
repository at:

`~/Code/oximod-d1-reverify-2026-08-09.tar.gz`

SHA-256:

`aaec4a8560c7751f4c17f48c678e78272d4aeac693b3fe373facb3a4972b64c3`

The archive contains the consumer projects, baseline/candidate lockfiles,
probe sources, command log, stdout/stderr captures, true exit-status files,
runner scripts, and the verifier report.
