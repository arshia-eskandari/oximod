# Maintainer provenance note — D4 / SR-3

External source-hidden re-verification determination:

`SR-3 = READY_TO_CLOSE`

Candidate implementation commit:

`e588274b8424fb63fb28b4c7fcbb01ef654cadc2`

READY_FOR_REVERIFY state-recording commit:

`956640ff959082bdb0f106f3a8a335db7aa911cf`

Important chronology note:

The source-hidden external re-verification was completed before the
READY_FOR_REVERIFY ledger transition was committed. The transition was present
only as remediation bookkeeping and was subsequently recorded in the commit
above.

The verifier consumed the candidate product source from the remediation
worktree. After recording the ledger transition, the maintainer ran:

`git diff --exit-code e588274b8424fb63fb28b4c7fcbb01ef654cadc2 956640ff959082bdb0f106f3a8a335db7aa911cf -- README.md Cargo.toml Cargo.lock oximod oximod_core oximod_macros`

Result:

`SOURCE_DIFF_EXIT=0`

This mechanically establishes that README, manifests, and all OxiMod
source/test trees were unchanged between the implementation commit and the
later READY_FOR_REVERIFY bookkeeping commit.

External evidence report SHA-256:

`6a96665122ae71e46460bd8dd60d8d7f034583ecc3692db608592853c5182fbf`

Archived external evidence:

`/home/arshia/Code/oximod-d4-reverify-2026-08-10.tar.gz`

Archive SHA-256:

`9a82aacfe15e6d0eec37c752f2149260291918d89010224f5e1e195c9574f3f0`

The source-hidden verifier intentionally did not inspect candidate Git history
or source files. Candidate commit provenance is therefore established by the
maintainer-side source comparison above rather than by the verifier.

The external report below is preserved verbatim.

---

# OxiMod D4 / SR-3 — source-hidden external re-verification

**Date:** 2026-08-10T01:54:44Z (canonical run) · **Verifier scope:** SR-3 only ·
**Server:** MongoDB 8.0.28, `mongodb://127.0.0.1:27019/?replicaSet=rs0` (rs0, sole member `localhost:27019`) ·
**Toolchain:** rustc/cargo 1.97.1

## 1. Source-hidden compliance statement

No file beneath `/home/arshia/Code/audit-remediation` was read, listed, grepped,
searched, stat'ed, or opened, and no git command was run against that
repository. The only interaction with the candidate path was Cargo consuming
`oximod = { path = "/home/arshia/Code/audit-remediation/oximod" }` from the
external consumer at `/tmp/oximod-d4-reverify/cand/consumer` (Cargo transitively
consumed the path-local `oximod_core` and `oximod_macros` the candidate crate
declares; their paths appear in `cargo tree` output only). All behavioral
knowledge came from: the frozen audit (read-only), consumer-side compiler
output, and runtime/server observation. The frozen audit was not modified.

**Limitation:** the maintainer-stated commit `e588274` could not be confirmed,
because confirming it would require a git command against the candidate
repository, which the source-hidden rule forbids. Candidate identity is pinned
by `cargo pkgid` (`path+file:///home/arshia/Code/audit-remediation/oximod#0.3.0`)
as observed at run time.

## 2. Baseline dependency identity

- `cargo pkgid oximod` → `registry+https://github.com/rust-lang/crates.io-index#oximod@0.3.0`
- `Cargo.lock`: `oximod 0.3.0`, `source = registry+…crates.io-index`,
  `checksum = 20e1bf01f5006f702e7a7319acced5d104dc8c3f0a1097a9f9a37457a9d18479`
  (likewise registry-sourced `oximod_core` / `oximod_macros` 0.3.0 with checksums).
- Separate lockfile and target dir (`base/target`). Evidence:
  `logs/base-pkgid.out`, `logs/base-tree.out`, `logs/base-lock-oximod.txt`.

## 3. Candidate dependency identity

- `cargo pkgid oximod` → `path+file:///home/arshia/Code/audit-remediation/oximod#0.3.0`
- `Cargo.lock`: `oximod 0.3.0` **with no `source`/`checksum` fields** (path
  dependency), same for `oximod_core`/`oximod_macros` — all resolved from the
  candidate checkout, none from crates.io.
- Separate lockfile and target dir (`cand/target`). Evidence:
  `logs/cand-pkgid.out`, `logs/cand-tree.out`, `logs/cand-lock-oximod.txt`.
- Cross-contamination check: baseline lock shows only registry sources;
  candidate lock shows only path sources. Observed candidate crate version is
  still `0.3.0` (not bumped) — version alone does not distinguish the two;
  the pkgid/lock source fields do.

## 4. Probe inventory

| Probe | Binary | Side | Expected | Observed |
|---|---|---|---|---|
| A: API differential | `a_api` (byte-identical both sides, sha256-verified) | base+cand | cand compiles / base rejected | cand exit 0 / base exit 101, E0599 ×2 |
| A: embedded boundary | `a_embedded` | cand | compile rejected | exit 101, E0599 ×2 |
| B: save-free establishment | `b_savefree` | cand | 4 specs, 0 docs | pass |
| C: explicit client, no global | `c_explicit` | cand | success, 0 docs | pass |
| D: unique before first save | `d_unique` | cand | typed update rejected | pass |
| E: repeated init | `e_repeat` | cand | 2×Ok, 1 index | pass |
| F(+I): shared once-state / no drift | `f_shared` | cand | no recreation | pass |
| G: failure + retry | `g_retry` | cand | fail, fail, fix, Ok | pass |
| H: lazy save-path regression | `h_lazy` (byte-identical both sides) | base+cand | identical lazy establishment | pass, outputs equivalent |

All server-state assertions were made through the raw `mongodb` driver inside
the probes **and** re-confirmed externally via `mongosh` inside the server
container (`logs/*-external.out`, `logs/final-sweep.out`). Both target
databases (`oximod_d4_sr3`, `oximod_d4_h`) were dropped and asserted empty
before the run (`logs/clean-drop.out` → `{"sr3":[],"h":[]}`).

## 5. Public-API compile differential (Probe A)

Byte-identical source (`probes/shared/a_api.rs`; sha256 `8796a695…` on both
sides) calling `AApi::init_indexes()` and
`AApi::init_indexes_from(&mongodb::Client)`:

- **Candidate:** `cargo build --bin a_api` exit 0.
- **Baseline 0.3.0:** exit 101 with
  `error[E0599]: no associated function or constant named 'init_indexes' found for struct 'AApi'`
  and the same for `init_indexes_from` (`logs/base-build-a_api.err`).

Criteria 1–3 satisfied. The observed signature is an associated async function
taking `&mongodb::Client` for the `_from` variant and returning
`Result<(), OxiModError>`-shaped values (`Ok(())` printed as `result=()`).

## 6. Embedded-model boundary

`#[model(embedded)]` struct on the **candidate**: both `init_indexes` and
`init_indexes_from` are rejected with E0599, originating in the `Model` derive
(`logs/cand-build-a_embedded.err`). Embedded models did not gain the API.
Criterion 4 satisfied.

## 7. Save-free establishment (Probe B)

Fresh, never-created collection `b_orders` (precondition: namespace absent,
NamespaceNotFound before the call; collection not in `listCollections`). No
`save()` anywhere in the binary. `BOrder::init_indexes()` → `Ok(())`. Immediately
after, raw `listIndexes` shows all four declared indexes; `count_documents` = 0.
Criterion 5 and 6 satisfied.

## 8. Server-side index specifications (fidelity)

Raw `listIndexes` verbatim after the save-free init (in-probe and external
snapshots agree):

| Declared attribute | Server spec observed |
|---|---|
| `#[index(unique, name="b_order_no_uidx")]` | `{"key":{"order_no":1},"name":"b_order_no_uidx","unique":true}` |
| `#[index(sparse, name="b_ref_sidx")]` | `{"key":{"ref_code":1},"name":"b_ref_sidx","sparse":true}` |
| `#[index(order=-1, name="b_priority_didx")]` | `{"key":{"priority":-1},"name":"b_priority_didx"}` |
| `#[index(expire_after_secs=120, name="b_ttl_idx")]` | `{"key":{"expires_at":1},"name":"b_ttl_idx","expireAfterSeconds":120}` |

TTL fidelity was proven from the stored `expireAfterSeconds` option, without
waiting for TTL deletion. Criterion 7 satisfied.

## 9. Zero-document proof

- Probe B: `doc_count=0` in-probe and `"count":0` external after establishment.
- Probe C: `doc_count=0` after `init_indexes_from`, and still 0 after the
  global-probe save failed.
- Probe E: `doc_count=0` after two inits.
- Probe D: total documents = 2, both raw-seeded, both externally listed with
  their seeded `_id`s — no OxiMod-authored document anywhere.
- Final sweep: every collection's count equals exactly the documents the probes
  deliberately wrote (`b_orders` 0, `c_items` 0, `d_users` 2 raw-seeded,
  `e_widgets` 0, `f_events` 1 from the intentional Probe F save, `g_shared` 0).

Index establishment writes no model documents. Criterion 6 satisfied,
adversarial check "secretly writing a model document" refuted.

## 10. Explicit-client result (Probe C)

In a process that never calls `OxiClient::init_global`,
`CItem::init_indexes_from(&client)` returned `Ok(())` and the raw client saw
`c_sku_uidx` (`unique:true`) with 0 documents. Adversarial confirmation that
the global was genuinely uninitialized: a subsequent `save()` in the same
process failed with
`GlobalClientMissing { msg: "Global MongoDB client has not been initialized" }`,
and the count stayed 0. The explicit-client path neither requires nor secretly
initializes the global client. Criterion 8 satisfied.

## 11. Unique-before-save result (Probe D)

Unique index established by `init_indexes()` alone; two documents seeded via
raw `insert_many` (no OxiMod write ever occurred in the process); then the
existing typed update `DUser::update_by_id(oid1002, {$set:{email:"a@example.com"}})`
was **rejected**: `Err(Database { … source: WriteError { code: 11000, message:
"E11000 duplicate key error … index: d_email_uidx …" } })`. Raw post-state:
`count(email=="a@example.com") == 1`, document 1002 still `b@example.com`,
total 2 — the duplicate did not persist. This is the exact A6/A9 window that
W3-F01/W3-V01 measured as silently violated on 0.3.0 (typed update returned
`Ok(true)` and created a real duplicate); on the candidate the window is closed
before any save. Criterion 9 satisfied.

## 12. Repeated-call result (Probe E)

`EWidget::init_indexes()` called twice in one process: both `Ok(())`, no
duplicate-index error, and exactly one `e_key_uidx` (plus `_id_`) on the
server. Criterion 10 satisfied.

## 13. Shared explicit-init/save once-state (Probe F)

After a successful `init_indexes()`, the index was dropped out-of-band via raw
`dropIndexes`. A subsequent ordinary `save()` in the same process succeeded
(`inserted_id=…0f01`) and did **not** recreate `f_key_uidx`
(`save_recreated_dropped_index=false`). Explicit init and the save path share
one completed once-per-process state — the authorized D4 lifecycle, recorded
as expected behavior, **not** a defect. Criterion 11 satisfied. (The "separate
Once state" adversarial hypothesis is refuted: had the states been separate,
the first save would have re-established the index.)

## 14. Failure/retry result (Probe G)

Two models sharing collection `g_shared` and index name `g_shared_idx` over
different fields (SR-7-clean conflict: each model's own declaration is valid):

1. `GAlpha::init_indexes()` → `Ok`, server shows `{alpha_key:1}`.
2. `GBeta::init_indexes()` → `Err(OxiModError::Index { … })`, underlying server
   error `code: 86, IndexKeySpecsConflict` — the existing index error surface.
3. Immediate second `GBeta::init_indexes()` → fails **again** with the same
   code-86 conflict — the failure retried against the server; it was neither
   memoised as success nor short-circuited.
4. Raw `dropIndexes` removed the conflict.
5. `GBeta::init_indexes()` → `Ok`; server now shows `{beta_key:1}` under
   `g_shared_idx` (in-probe and external snapshots agree).

Failed initialization is not remembered as successful and is not permanently
poisoned. Criterion 12 satisfied.

## 15. Lazy save-path regression (Probe H)

Byte-identical binary (sha256 `cd298c86…` both sides) that never calls
`init_indexes*()`: namespace absent before and after `init_global`; first
`save()` establishes `h_key_uidx` (`unique:true`) with exactly 1 document —
**identical observable behavior on baseline 0.3.0 and candidate** (external
snapshots byte-equivalent). Existing applications retain lazy save-triggered
establishment. Criterion 13 satisfied.

## 16. Drift/re-establishment negative boundary (Probe I, in `f_shared`)

After the out-of-band drop and the non-recreating save, a further
`init_indexes()` call in the same process returned `Ok(())` and still did
**not** recreate the dropped index (`reinit_recreated_dropped_index=false`;
final sweep confirms `f_events` has only `_id_`). D4 did not turn
`init_indexes()` into syncIndexes-style drift repair. Criterion 14 satisfied.

## 17. Unexpected results

None that affect the verdict. Three observations worth recording:

1. The candidate crate still reports version `0.3.0` despite the added public
   API (a semver-minor surface) — a release-hygiene note for the maintainer,
   not an SR-3 behavior defect, and out of scope here.
2. Probe D's duplicate rejection surfaces as `OxiModError::Database` (wrapping
   the E11000 `WriteError`) on the update path, while Probe G's `createIndexes`
   conflict surfaces as `OxiModError::Index`. Both are pre-existing 0.3.0 error
   surfaces for those operations (consistent with frozen-audit observations);
   noted for completeness.
3. A later `init_indexes()` after the once-state completes returns `Ok(())`
   even though it re-establishes nothing (Probes E/F). This is the documented
   once-per-process contract being tested, recorded as expected.

## 18. Artifact map

```
/tmp/oximod-d4-reverify/
├── REPORT.md                  — this report
├── run.sh                     — canonical logged evidence run (37 logged commands)
├── base/consumer/             — baseline consumer (=0.3.0 crates.io), own Cargo.lock
├── base/target/               — baseline-only build cache
├── cand/consumer/             — candidate consumer (path dep), own Cargo.lock
├── cand/target/               — candidate-only build cache
├── probes/shared/             — byte-identical sources (lib.rs, a_api.rs, h_lazy.rs)
├── probes/cand-only/          — a_embedded, b_savefree, c_explicit, d_unique,
│                                e_repeat, f_shared, g_retry
└── logs/
    ├── commands.txt           — every logged command, in order (37)
    ├── env.txt, env-*.out     — toolchain, server 8.0.28, rs0 conf, ping
    ├── clean-drop.out         — pre-run virgin-state proof
    ├── base-build-*.{out,err,exit}, base-run-h_lazy.*, base-tree.out,
    │   base-pkgid.out, base-lock-oximod.txt, base-h-external.out
    ├── cand-build-*.{out,err,exit}, cand-run-*.{out,err,exit}, cand-tree.out,
    │   cand-pkgid.out, cand-lock-oximod.txt, cand-[b-h]-external.out
    ├── shared-source-identity.txt — sha256 identity of shared probe sources
    └── final-sweep.out        — external end-state of every collection
```

Every `.exit` file records the true exit status; expected-failure compiles
(`base-build-a_api`, `cand-build-a_embedded`) recorded exit 101 with the E0599
stderr preserved. `run.sh` uses `set -o pipefail`.

## 19. Measured command count

37 logged commands in the canonical `run.sh` execution
(`wc -l logs/commands.txt` = 37: 3 env + 1 clean + 12 build/identity + 2 lock
excerpts recorded inline + 10 probe runs + 9 external mongosh verifications),
plus the interactive pre-flight session (toolchain check, audit reading,
initial compile validation of the two expected-failure probes) whose results
were all re-executed and re-captured by `run.sh`.

## 20. Final determination

All 14 pass criteria are satisfied, each with in-process raw-driver evidence
and an independent external mongosh confirmation:

1. ✔ `init_indexes()` on collection models (candidate)
2. ✔ `init_indexes_from(&client)` on collection models (candidate)
3. ✔ Baseline `=0.3.0` exposes neither (E0599 ×2)
4. ✔ Embedded models receive neither (E0599 ×2 on candidate)
5. ✔ Save-free establishment of declared indexes
6. ✔ Zero model documents written by establishment
7. ✔ Server-side fidelity: unique, sparse, order −1, TTL 120 s
8. ✔ Explicit client works with no global client (and global proven absent)
9. ✔ Unique enforcement active before first OxiMod save (E11000 on typed update)
10. ✔ Repeated successful init harmless (2×Ok, one index)
11. ✔ Explicit init and save share one completed once-state
12. ✔ Failed init retries and succeeds after conflict removal (code 86 → Ok)
13. ✔ Lazy save-path establishment unregressed (byte-identical differential)
14. ✔ No drift synchronization added (dropped index never recreated)

**SR-3 = READY_TO_CLOSE**
