# OxiMod Audit Remediation — D6 External Reverification

## Status

SR-2 = READY_TO_CLOSE

## Scope

Final source-hidden external differential re-verification for D6 / SR-2
against crates.io OxiMod 0.3.0, with frozen-audit target W1-V07.

## Provenance bridge

The external verifier intentionally operated as a source-hidden consumer.

It consumed the candidate only through the Cargo path dependency:

  /home/arshia/Code/audit-remediation/oximod

The verifier did not inspect candidate Git history or candidate source and
therefore did not claim a candidate Git commit.

The maintainer establishes repository provenance separately:

- D6 implementation commit:
  0dfd78b4ac91a5f744cee9100f2e6ebe9d515cf3

- D6 READY_FOR_REVERIFY commit:
  6a69d22f9803fa0764f744d775bea2baeb3d153e

- repository HEAD during the provenance check:
  6a69d22f9803fa0764f744d775bea2baeb3d153e

- product-source diff from implementation commit to READY commit:
  SOURCE_DIFF_EXIT=0

A SOURCE_DIFF_EXIT of 0 proves that the product source represented by the
READY commit is identical to the reviewed D6 implementation source. The
READY commit contains bookkeeping only.

## External result

SR-2 = READY_TO_CLOSE

All required A-M probe families and all ten approved SR-2 semantic assertions
passed.

The verifier reproduced the historical 0.3.0 W1-V07 inconsistencies on the
baseline and observed exactly the authorized failure-class remaps on the
candidate, with no unexplained candidate differences.

The original mongodb::error::Error remained available through source(), and
duplicate-key server code 11000 remained recoverable.

## Evidence integrity

Report SHA-256:

  7dfc89f4ce0b028dbc7eeda049d76cf33fc0b4c23457da980a10c65bbf871c78

Evidence archive:

  /home/arshia/Code/oximod-d6-reverify-2026-08-10.tar.gz

Archive SHA-256:

  170bdf4d1c599d660d0b648f7421e2e66e6e4dc5ba013c0774507b40714c3515

Archive readability:

  ARCHIVE_OK

## Verifier report

# OxiMod D6 / SR-2 — Final Source-Hidden External Differential Re-Verification

- Date: 2026-08-10
- Frozen-audit target: W1-V07
- Scope: EXTERNAL VERIFICATION ONLY (black-box consumer)
- Verdict: **SR-2 = READY_TO_CLOSE** (see §8)

## 1. Environment

| Item | Value |
|---|---|
| rustc | 1.97.1 (8bab26f4f 2026-07-14) |
| cargo | 1.97.1 (c980f4866 2026-06-30) |
| MongoDB server | 8.0.28, replica set `rs0`, PRIMARY, `mongodb://127.0.0.1:27019/?replicaSet=rs0` (container `oximod-blackbox-mongodb`, image `mongo:8.0`) |
| OS | Linux 6.8.0-106-generic |

## 2. Subjects under test

**Baseline** (workspace `/tmp/oximod-d6-reverify/base`, own `Cargo.lock` + `target/`):

- `oximod 0.3.0` — `registry+https://github.com/rust-lang/crates.io-index`, checksum `20e1bf01f5006f702e7a7319acced5d104dc8c3f0a1097a9f9a37457a9d18479`
- `oximod_core 0.3.0` — checksum `8057d4e075eef42cb8ec9abab2f3b82a15301224c463ba758e82c0914778b905`
- `oximod_macros 0.3.0` — checksum `e6715e9f3f5077c3ceee58bbd78c2fbb95df2c403226844a0cc58409f72abc93`
- resolves `mongodb 3.8.0`, `bson 2.15.0`

**Candidate** (workspace `/tmp/oximod-d6-reverify/cand`, own `Cargo.lock` + `target/`):

- `oximod = { path = "/home/arshia/Code/audit-remediation/oximod" }` — consumed ONLY as a Cargo
  path dependency. Cargo resolved it as `oximod 0.3.0` (path), pulling in path-local
  `oximod_core 0.3.0` and `oximod_macros 0.3.0`, and `mongodb 3.8.0` / `bson 2.15.0` from
  crates.io (same driver versions as baseline).
- No candidate Git commit is claimed: Git/source inspection was intentionally forbidden.
  The maintainer bridges source provenance separately.

Lockfile/target separation: `base/Cargo.lock` ≠ `cand/Cargo.lock`; `base/target/` (2.0 GB) and
`cand/target/` (2.2 GB) are fully independent build trees.

## 3. Source-hidden compliance

- The candidate repository's source, Git history, manifests, `Cargo.lock`, and remediation
  control files were never read, listed, searched, or statted by the verifier.
- No `git` command was run against the candidate repository. No `cargo expand`; no
  `cargo metadata` was used to discover candidate implementation details.
- The candidate was consumed exclusively through Cargo's path-dependency mechanism.
- The public API surface was learned from (a) the published crates.io `oximod 0.3.0` /
  `oximod_core 0.3.0` / `oximod_macros 0.3.0` sources in the local cargo registry cache
  (baseline artifacts, permitted), and (b) rustc compile diagnostics against the candidate
  as an ordinary consumer (used only to establish that `init_indexes` / `init_indexes_from`
  exist on the candidate and are absent on the baseline).
- All verifier work lives outside the candidate repository, under `/tmp/oximod-d6-reverify`.
- Classification is performed ONLY by matching public `OxiModError` variants
  (`classify()` in `probes/main.rs`). Display strings are recorded as evidence but never
  used for classification.

## 4. Method

One probe program (`probes/main.rs`) is compiled unmodified against both subjects
(`base/src/main.rs` and `cand/src/main.rs` are byte-identical copies). Candidate-only public
API (`init_indexes`, `init_indexes_from`) is exercised behind an opt-in cargo feature
`init_indexes`; enabling that feature on the baseline fails to compile with E0599
(`no associated function ... named init_indexes`), which is the documented baseline
limitation for probe J (baseline 0.3.0 has no public index-init entry point; its index
creation runs lazily inside `save`).

For every failing operation the probe records:

- the `OxiModError` variant (variant match only);
- whether `std::error::Error::source()` directly downcasts to `mongodb::error::Error`
  (type identity guaranteed via the crate's own `_mongodb` re-export);
- the full source chain by downcast identity, with the driver `ErrorKind`;
- whether duplicate-key server code 11000 is recoverable from the driver error, and through
  which representation (`ErrorKind::Write(WriteFailure::WriteError)` vs `ErrorKind::Command`).

Connectivity probes use a closed port with short timeouts
(`mongodb://127.0.0.1:9/?serverSelectionTimeoutMS=300&connectTimeoutMS=300`). Live probes
drop the `sr2_reverify` database before each run. Modes: `live` (working global client +
explicit dead client for `_from` probes), `down` (global client aimed at dead endpoint),
`noclient` (no global client). All six runs (2 subjects × 3 modes) exited 0; raw outputs in
`logs/`. Canonical commands in `run.sh` and at the top of each log; every log ends with the
true `EXIT=` status.

## 5. Differential probe matrix

Notation: `variant / direct-source / 11000` from `RESULT|` lines in `logs/*.log`.
"mongo(X)" = `source()` downcasts directly to `mongodb::error::Error` with `ErrorKind` X.

| Probe | Operation | Baseline observed | Candidate observed | Candidate expected | Pass |
|---|---|---|---|---|---|
| A | `save` duplicate key | **Connection** / mongo(Write code=11000) / 11000=Write | **Database** / mongo(Write code=11000) / 11000=Write | Database, source downcast, 11000 recoverable | ✅ |
| B | `update_by_id` duplicate key | Database / mongo(Write 11000) / Write | Database / mongo(Write 11000) / Write | Database, 11000 recoverable | ✅ |
| C | typed `update_one` duplicate key | Database / mongo(Command code=11000) / Command | Database / mongo(Command code=11000) / Command | Database; 11000 recoverable (surfaced via **Command**, from `find_one_and_update`) | ✅ |
| D | `save` client-side BSON encode failure | **Connection** / mongo(BsonSerialization) | **Serialization** / mongo(BsonSerialization) | Serialization, source preserved (both sides preserve driver source) | ✅ |
| E | `save_from` unreachable | Connection / mongo(ServerSelection) | Connection / mongo(ServerSelection) | Connection | ✅ |
| F | `find_by_id_from` unreachable | Database | **Connection** / mongo(ServerSelection) | Connection | ✅ |
| F | `update_by_id_from` unreachable | Database | **Connection** | Connection | ✅ |
| F | `delete_by_id_from` unreachable | Database | **Connection** | Connection | ✅ |
| F | `exists_from` unreachable | Database | **Connection** | Connection | ✅ |
| F | `count_from` unreachable | Database | **Connection** | Connection | ✅ |
| F | `clear_from` unreachable | Database | **Connection** | Connection | ✅ |
| G | typed `first()` unreachable | Database | **Connection** | Connection | ✅ |
| G | typed `all()` unreachable | Database | **Connection** | Connection | ✅ |
| G | typed `count()` unreachable | Database | **Connection** | Connection | ✅ |
| G | typed `delete_one()` unreachable | Database | **Connection** | Connection | ✅ |
| G | typed `delete_all()` unreachable | Database | **Connection** | Connection | ✅ |
| G | typed `update_one()` unreachable | Database | **Connection** | Connection | ✅ |
| G | typed `update_all()` unreachable | Database | **Connection** | Connection | ✅ |
| G2 | global `save` unreachable | Connection | Connection | Connection | ✅ |
| G2 | global `find_by_id` unreachable | Database | **Connection** | Connection | ✅ |
| H | `find_by_id` malformed doc | **Database** / mongo(BsonDeserialization) | **Serialization** / mongo(BsonDeserialization) | Serialization, source downcast, BsonDeserialization observable | ✅ |
| H | typed `first()` malformed doc | **Database** / mongo(BsonDeserialization) | **Serialization** / mongo(BsonDeserialization) | Serialization | ✅ |
| H | typed `all()` malformed doc | Serialization / mongo(BsonDeserialization) | Serialization / mongo(BsonDeserialization) | Serialization | ✅ |
| I | index spec conflict via `save` | Index / mongo(Command code=86) | Index / mongo(Command code=86) | Index, driver source preserved | ✅ |
| I2 | index spec conflict via `init_indexes()` | n/a (API absent, E0599) | Index / mongo(Command code=86) | Index | ✅ |
| J | indexed `save_from` unreachable (closest baseline scenario) | **Index** / mongo(ServerSelection) | **Connection** / mongo(ServerSelection) | Connection | ✅ |
| J2 | `init_indexes_from` / global `init_indexes` unreachable | n/a (API absent, E0599) | Connection / mongo(ServerSelection) (both call sites) | Connection | ✅ |
| K | `save` / `find_by_id` / typed `all()` with no global client | GlobalClientMissing (all three) | GlobalClientMissing (all three) | GlobalClientMissing | ✅ |
| L | validation failure (`min_length`) via `save` | Validation | Validation | Validation (no driver remap) | ✅ |
| M | typed query `page(0, …)` | Query | Query | Query (no Database/Connection remap) | ✅ |
| A0 | control: first unique save | ok | ok | ok | ✅ |

Baseline observations match the historical W1-V07 record everywhere the frozen audit
specified one (A→Connection, B→Database, D→Connection, E→Connection, F→Database,
H→Database/Database/Serialization, I→Index, J→Index).

## 6. Semantic assertions

1. **Duplicate key is Database regardless of save/update call site** — A, B, C all Database on candidate. ✅
2. **Operation-time connectivity failures are Connection regardless of CRUD/query call site** — E, F(×6), G(×7), G2(×2), J, J2 all Connection on candidate (16/16 call sites). ✅
3. **BSON decode failures are Serialization regardless of read terminal** — H `find_by_id`, `first()`, `all()` all Serialization. ✅
4. **BSON encode failure during save is Serialization** — D. ✅
5. **Non-connectivity index rejection remains Index** — I (via save) and I2 (via `init_indexes()`), server code 86, driver source preserved. ✅
6. **Index connectivity failure is Connection** — J, J2 (three distinct call paths). ✅
7. **`source()` preserves the original MongoDB driver error** — every source-backed candidate failure above shows a *direct* `source()` downcast to `mongodb::error::Error` (column "direct"); none required chain-walking. ✅
8. **Duplicate-key code 11000 remains recoverable** — recovered from the downcast driver error in A, B (as `ErrorKind::Write(WriteError)`) and C (as `ErrorKind::Command`, from `find_one_and_update`); per instructions no single representation is required. ✅
9. **GlobalClientMissing, Validation, and Query remain distinct** — K, L, M unchanged, no driver classification replaces them. ✅
10. **No unexpected candidate mapping difference beyond approved SR-2 changes** — the complete set of baseline↔candidate differences is: A (Connection→Database), D (Connection→Serialization), F ×6 and G ×7 and G2 `find_by_id` (Database→Connection), H `find_by_id`/`first()` (Database→Serialization), J (Index→Connection), plus the candidate-only additive public API `init_indexes`/`init_indexes_from`. Every one is an approved SR-2 change or an additive API; all other probes (A0, B, C, E, G2 save, H `all()`, I, K, L, M) are byte-for-byte classification-identical. ✅

Per instructions, no retry-safety or did-the-operation-reach-MongoDB semantics were inferred
from the Connection variant; that is outside the SR-2 contract.

## 7. Baseline limitations (documented honestly)

- Baseline 0.3.0 exposes no public `init_indexes()` / `init_indexes_from()`; compiling the
  candidate-only probes against baseline fails with E0599 (evidence in §3). The closest
  source-hidden baseline scenario — index creation triggered lazily by `save_from` against an
  unreachable deployment — was used for the probe-J baseline column and reproduces the
  historical `Index` classification for an index-path connectivity failure.
- Probe C's duplicate-key surfaces as `ErrorKind::Command(code=11000)` (driver
  `find_one_and_update` behavior) rather than `ErrorKind::Write` on **both** subjects; the
  contract requirement (11000 recoverable) holds in both representations.

## 8. Verdict

All required differential probes (A–M) produced the expected candidate classifications; all
ten semantic assertions pass; baseline limitations are documented; every observed
baseline↔candidate difference is within the approved SR-2 change set.

**SR-2 = READY_TO_CLOSE**

(External verdict only. SR-2 is NOT marked closed by this report; source provenance bridging,
closure, and release preparation remain with the maintainer.)

## 9. Evidence inventory

- `run.sh` — canonical build/run driver (all commands, true exit statuses echoed as `EXIT=`).
- `probes/main.rs` — canonical probe source (byte-identical to `base/src/main.rs` and `cand/src/main.rs`).
- `base/` — baseline workspace (Cargo.toml, Cargo.lock, src; `target/` excluded from archive).
- `cand/` — candidate workspace (Cargo.toml, Cargo.lock, src; `target/` excluded from archive).
- `logs/build-base.log`, `logs/build-cand.log` — build evidence.
- `logs/base-{live,down,noclient}.log`, `logs/cand-{live,down,noclient}.log` — raw probe output (`RESULT|` lines), each with command line and `EXIT=0`.
