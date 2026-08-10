# Maintainer provenance note — D3 / SR-8

External source-hidden re-verification determination:

SR-8 = READY_TO_CLOSE

Candidate implementation commit:

`9ef97c4335b802c74ad592ca539f5fb6a9e4aefc`

Post-implementation ledger-state commit used by the external candidate:

`498233f`

Maintainer provenance check:

`git diff --exit-code 9ef97c4335b802c74ad592ca539f5fb6a9e4aefc 498233f -- README.md Cargo.toml Cargo.lock oximod oximod_core oximod_macros`

Result:

`SOURCE_DIFF_EXIT=0`

This mechanically establishes that the implementation, tests, public
documentation, and manifests used by the path candidate were unchanged
between the D3 implementation commit and the READY_FOR_REVERIFY ledger commit.

External evidence report SHA-256:

`7b27cda84857003e12115ae8b5d0ab9ee5555b7514e7c1584da796cd16ec5704`

Archived external evidence:

`/home/arshia/Code/oximod-d3-reverify-2026-08-09.tar.gz`

Archive SHA-256:

`0dd3e3410ece77ab4053307381a2b01c9918dc0ba42727d03ca97779743447a0`

The source-hidden verifier intentionally did not inspect Git or candidate
source and therefore recorded the implementation SHA as maintainer-stated.
The maintainer provenance check above bridges that deliberate isolation.

The external report below is preserved verbatim. Its own date header is not
used as technical evidence for the SR-8 determination.

---

# SR-8 — Source-Hidden External Re-Verification (OxiMod D3)

- **Date:** 2026-08-09 (UTC), toolchain rustc/cargo 1.97.1, Linux 6.8.0-106-generic
- **Scope:** exactly SR-8 — ordered and numeric/modulo operators on `Field<Option<T>>`
  with INNER-TYPE operands. SR-2/SR-3, CLOSED SR items and P-items not assessed
  (one required negative boundary probe against the excluded bitwise Option
  expansion only).
- **Maintainer-stated implementation commit:** `9ef97c4335b802c74ad592ca539f5fb6a9e4aefc`
  (recorded as stated; NOT verified — verifying it would require git access to the
  remediation repository, which the source-hidden rule forbids).
- **Maintainer-stated remediation HEAD:** `498233f` (same caveat).

## Final determination

**SR-8 = READY_TO_CLOSE**

All twelve mandatory pass criteria were observed to hold. Details below.

---

## 1. Source-hidden compliance statement

No file under `/home/arshia/Code/audit-remediation` was read, listed, grepped,
searched, stat-ed or otherwise inspected at any point. No git command was run
against the remediation repository. The only interaction with that path was
Cargo/rustc compiling it as a path dependency from the external consumer project
`/tmp/oximod-d3-reverify/cand`. Everything asserted about the candidate below is
derived exclusively from:

- downstream compiler diagnostics of externally-authored consumer code;
- runtime behaviour of externally-authored consumer binaries against MongoDB;
- dependency-resolution identity (Cargo.lock / `cargo tree -i oximod`);
- the frozen audit repository (read-only), used solely for SR-8 evidence and
  public-API idioms already recorded there.

The frozen audit was not modified. Nothing was committed or pushed.

## 2. Candidate dependency identity

From `cand/Cargo.lock` and `logs/cand-tree-oximod.txt`:

```
oximod v0.3.0 (/home/arshia/Code/audit-remediation/oximod)
└── sr8-cand v0.1.0 (/tmp/oximod-d3-reverify/cand)
```

Lockfile entries for `oximod`, `oximod_core`, `oximod_macros` all version 0.3.0
with **no `source` field and no checksum** — the signature of a path dependency.
The candidate does not resolve from the registry.

## 3. Baseline dependency identity

From `base/Cargo.lock` and `logs/base-tree-oximod.txt`:

```
oximod 0.3.0  source = "registry+https://github.com/rust-lang/crates.io-index"
              checksum = 20e1bf01f5006f702e7a7319acced5d104dc8c3f0a1097a9f9a37457a9d18479
oximod_core 0.3.0   checksum = 8057d4e075eef42cb8ec9abab2f3b82a15301224c463ba758e82c0914778b905
oximod_macros 0.3.0 checksum = e6715e9f3f5077c3ceee58bbd78c2fbb95df2c403226844a0cc58409f72abc93
```

The baseline does not resolve the candidate path. Workspaces, lockfiles and
`target/` directories are fully separate.

## 4. Probe inventory

One canonical probe set (`probes/bin/*.rs`) copied **byte-identically** into both
consumers (`logs/probe-sha256.txt` proves identity). 19 binaries:

| Probe | Purpose | Expected base | Expected cand |
| --- | --- | --- | --- |
| `m00_model_smoke` | derive + already-Option-transparent families control | PASS | PASS |
| `a1_opt_i64_ordered` | `Option<i64>` gt/gte/lt/lte, inner operands | FAIL | PASS |
| `a2_opt_datetime_ordered` | `Option<DateTime>` gte/lt, inner operands | FAIL | PASS |
| `a3_opt_string_ordered` | `Option<String>` lt, owned inner operand | FAIL | PASS |
| `a3b_opt_string_str_operand` | informational: `lt("m")` &str spelling | FAIL | recorded |
| `b1_opt_i32_numeric` | `Option<i32>` inc/mul/min/max/modulo, inner operands | FAIL | PASS |
| `b2_opt_f64_numeric` | `Option<f64>` inc/mul/min/max, inner operands | FAIL | PASS |
| `c1_gt_none` … `c4_inc_some` | REQUIRED negative: None / Some(..) operands | FAIL | **FAIL** |
| `d1_bool_ordered` | REQUIRED negative: ordered on bool / `Option<bool>` | FAIL | **FAIL** |
| `d2_opt_bitwise` | REQUIRED negative: `Option<i32>.bits_all_set` | FAIL | **FAIL** |
| `e1_required_regression` | required-field ordered/numeric/bitwise/composition surface | PASS | PASS |
| `e2_set_coherence` | F-V01 `.set()` operand convention unchanged | PASS | PASS |
| `rt_e_opt_i64` | MongoDB workflow E (also baseline compile control) | FAIL | PASS+run |
| `rt_f_opt_datetime` | MongoDB workflow F (ditto) | FAIL | PASS+run |
| `rt_g_opt_i32_update` | MongoDB workflow G (ditto) | FAIL | PASS+run |
| `rt_h_modulo` | MongoDB workflow H (ditto) | FAIL | PASS+run |

Runner: `run.sh` (deterministic, `set -u -o pipefail`, true exits recorded per
command in `logs/summary.txt`; every command's stdout/stderr preserved as
`logs/<side>-<mode>-<bin>.{stdout,stderr}`).

## 5. Ordered compile-matrix results (criteria 1, 2) — PASS

Observed differential matches expected differential on every probe:

| Probe | base `cargo check` | cand `cargo check` |
| --- | --- | --- |
| a1 (`Option<i64>` ×4 ops) | exit 101 | exit 0 |
| a2 (`Option<DateTime>` ×2 ops) | exit 101 | exit 0 |
| a3 (`Option<String>` lt) | exit 101 | exit 0 |
| a3b (informational &str operand) | exit 101 | exit 0 |

Baseline rejections are the exact frozen-audit diagnostic (per-operator E0599,
`Option<T>: OrderedQueryValue` unsatisfied), reproducing W2-F01/W2-V01 verbatim —
so the baseline failure is the SR-8 gap itself, not an unrelated breakage.

## 6. Numeric/modulo compile-matrix results (criteria 3, 4) — PASS

| Probe | base | cand |
| --- | --- | --- |
| b1 (`Option<i32>` inc/mul/min/max/modulo) | exit 101 (E0599 ×5, `NumericQueryValue`) | exit 0 |
| b2 (`Option<f64>` inc/mul/min/max) | exit 101 (E0599 ×4, `NumericQueryValue`) | exit 0 |

## 7. None/Some negative-boundary results (criterion 5) — PASS

All four REQUIRED candidate compile failures observed, with operand-shape
diagnostics (not method-absence), proving the `Option` wrapper belongs to the
FIELD type and operands remain INNER values:

| Probe | cand exit | cand diagnostic |
| --- | --- | --- |
| `optional_i32.gt(None)` | 101 | E0277 `i32: From<Option<_>>` not satisfied |
| `optional_i32.gt(Some(18))` | 101 | E0277 `i32: From<Option<{integer}>>` not satisfied |
| `optional_i32.inc(None)` | 101 | E0277 `i32: From<Option<_>>` not satisfied |
| `optional_i32.inc(Some(1))` | 101 | E0277 `i32: From<Option<{integer}>>` not satisfied |

(The E0277 `From` shape, together with a3b compiling `lt("m")` on
`Option<String>`, indicates the candidate operand position is
into-inner-converting — it accepts `T` or things convertible to `T`, never
`Option<T>`. This is an observation from downstream diagnostics only.)

Baseline also rejects all four (E0599, method not callable at all) — recorded
for completeness in `logs/base-check-c*.stderr`.

## 8. bool and bitwise negative boundaries (criteria 6, 7) — PASS

- `d1_bool_ordered`: cand exit 101 — E0599 `gt` unsatisfied on **both**
  `Field<bool>` and `Field<Option<bool>>`. Ordered comparison on bool remains
  rejected; no unconditional blanket implementation exists.
- `d2_opt_bitwise`: cand exit 101 — E0599 `bits_all_set` unsatisfied on
  `Field<Option<i32>>`; the explicitly excluded bitwise/integer-only Option
  expansion was NOT accidentally implemented. Positive control: bare-`i32`
  `bits_all_set` compiles on the candidate inside `e1_required_regression`
  (exit 0). No broader P-7 testing was performed.

## 9. Option<i64> MongoDB workflow (criterion 8) — PASS

`rt_e_opt_i64` on candidate, DB `oximod_d3_reverify`, unique collection
`rt_e_leases`, dropped before and after. Seeded: 100 (typed save), 500 (typed
save), explicit BSON `null` (raw driver), field-absent row (raw driver);
SEED_TOTAL=4 confirms no stale rows.

- Typed inner-operand queries: `gt(300)`→[2], `gte(100)`→[1,2], `lt(300)`→[1],
  `lte(500)`→[1,2], composed band `gte(300) & lt(600)`→[2] — all as expected.
- Null/missing rows matched **no** ordered predicate (never treated as numeric).
- Raw-driver oracle agreed on all four operators; typed `count()` agreed.
- Exit 0, `RESULT=PASS`.

Baseline control: the identical file fails to compile under =0.3.0 with the four
`OrderedQueryValue` E0599s (`logs/base-check-rt_e_opt_i64.stderr`).

## 10. Option<DateTime> MongoDB workflow (criterion 9) — PASS

`rt_f_opt_datetime` on candidate, collection `rt_f_events`: rows at 1000ms
(before range), 5000ms (in range), missing. Typed inner `DateTime` operands:
range `[2000,9000)`→[2], `lt(2000)`→[1], `gte(2000)`→[2]; missing row excluded
everywhere; raw oracle agreed. Exit 0, `RESULT=PASS`. Baseline control: compile
failure under =0.3.0 (`logs/base-check-rt_f_opt_datetime.stderr`).

## 11. Option<i32> numeric update workflow (criterion 10) — PASS

`rt_g_opt_i32_update` on candidate, collection `rt_g_counters`, counter starts
`Some(10)`, second untouched row (777) as over-broad-update guard:

| Step | typed readback | raw oracle | expected |
| --- | --- | --- | --- |
| `inc(5)` | Some(15) | Some(15) | 15 ✓ |
| `mul(2)` | Some(30) | Some(30) | 30 ✓ |
| `min(25)` | Some(25) | Some(25) | 25 ✓ |
| `max(40)` | Some(40) | Some(40) | 40 ✓ |

Bystander row unchanged (typed and raw agree on 777). Exit 0, `RESULT=PASS`.
Typed readback and raw MongoDB state agreed after every step. Baseline control:
compile failure under =0.3.0.

## 12. Modulo runtime smoke (criterion 11) — PASS

`rt_h_modulo` on candidate, collection `rt_h_modulo`: values 4, 7, BSON null,
missing. `modulo(2,0)`→[key of 4], `modulo(2,1)`→[key of 7]; null/missing
excluded from both; raw `$mod` oracle agreed. Exit 0, `RESULT=PASS`. Baseline
control: compile failure under =0.3.0.

## 13. Required-field regression control (criterion 12) — PASS

`e1_required_regression` (bare i64/f64/i32/String/DateTime ordered ops, `&`
composition, `.not()` negation, i32/f64 inc/mul/min/max, modulo, bare-i32
`bits_all_set`) compiles cleanly on BOTH sides (exit 0 / exit 0).
`e2_set_coherence` confirms the F-V01-measured `.set()` operand convention is
unchanged on the candidate: `set(Some(5))`, `set(None)`, `set(5)` on
`Option<i32>` and `set(7)` on required `i32` all compile on both sides.
`m00_model_smoke` confirms the pre-existing Option-transparent families
(eq/exists/is_null/is_not_null/asc/starts_with) still compile on both sides.
No evidence of any required-field or pre-existing-surface regression.

## 14. Unexpected results (all disclosed)

1. **First-run probe defect (mine, not the candidate's).** In run 1,
   `rt_e`/`rt_f`/`rt_h` failed to compile on the candidate with **E0503**
   ("cannot use `ok` because it was mutably borrowed") — a borrow-checker bug in
   my own probe harness (a closure capturing `ok` mutably while `ok` was also
   assigned directly). The probes were rewritten to use a free function and the
   whole suite re-run; run 2 is the canonical evidence. Run 1's summary is
   archived at `logs/run1-summary-archived.txt`. Run 1's per-bin stderr files
   were overwritten by run 2 (disclosed; the E0503 diagnostic is quoted here and
   the archived summary shows the exit pattern). Every SR-8-relevant differential
   that could compile in run 1 (a*, b*, c*, d*, e1, rt_g) already showed the same
   outcome as run 2.
2. **Informational:** `a3b` (`optional_string.lt("m")`, `&str` operand) compiles
   on the candidate. This is a convenience-conversion acceptance of a
   borrowed/convertible-to-inner operand, not an `Option` operand; the mandatory
   None/Some rejections (§7) bound it. Recorded, not judged.
3. The candidate reports its package version as `0.3.0` (same as the published
   baseline). Not an SR-8 criterion; noted for release hygiene since a crate
   shipping this change would need a version bump.

Nothing else deviated from expectation. No evidence of: Option operand
acceptance, unconditional blanket marker impls (bool rejected), bitwise Option
support, required-field regression, typed-vs-raw runtime mismatch (oracle agreed
in all 4 workflows), wrong dependency resolution on either side, or stale rows
(collections dropped before/after; seed counts exact).

## 15. Artifact map

```
/tmp/oximod-d3-reverify/
  REPORT.md                     this report
  run.sh                        reproducible runner (canonical)
  probes/bin/*.rs               canonical probe sources (19 binaries)
  base/                         baseline consumer (oximod =0.3.0, own Cargo.lock, own target/)
  cand/                         candidate consumer (path dep, own Cargo.lock, own target/)
  logs/summary.txt              per-command true exit codes (run 2, canonical)
  logs/run1-summary-archived.txt  run-1 record (pre probe-fix)
  logs/probe-sha256.txt         byte-identity of probe sources across base/cand
  logs/base-lock-oximod.txt     baseline registry identity + checksums
  logs/cand-lock-oximod.txt     candidate path-dep identity
  logs/{base,cand}-tree-oximod.txt  cargo tree -i oximod
  logs/<side>-check-<bin>.{stdout,stderr}  per-probe compile evidence
  logs/cand-run-<bin>.{stdout,stderr}      runtime workflow evidence
  logs/environment.txt          toolchain/OS/URI record
```

## 16. Command count

Canonical run (run 2): **46 measured commands** recorded with true exit status in
`logs/summary.txt` (2 lockfile generations, 2 dependency-tree queries, 38 compile
checks = 19 probes × 2 sides, 4 candidate runtime executions). Run 1 recorded 48
command entries (archived). Ancillary evidence-gathering commands (log greps,
sha256, environment capture) are read-only and preserved in the artifact tree.

## 17. Determination

All 12 mandatory pass criteria observed true. **SR-8 = READY_TO_CLOSE.**

(Scope note: this closes only the SR-8 surface as specified. The excluded
bitwise Option family remains unimplemented by design — reconfirmed in §8 — and
was not broadened into P-7 testing.)
