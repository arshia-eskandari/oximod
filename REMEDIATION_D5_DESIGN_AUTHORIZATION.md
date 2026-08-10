# OxiMod Audit Remediation — D5 Error-Contract Design Authorization

## Status

D5 DESIGN AUTHORIZED — SR-2 ONLY — IMPLEMENTATION FORBIDDEN

## Purpose

D5 designs the final pre-1.0 remediation item:

- SR-2 — coherent OxiModError semantics

This phase MUST NOT modify product source, tests, public documentation,
Cargo manifests, Cargo.lock, or the remediation ledger.

Its output is a design proposal for maintainer review.

Implementation requires a separate maintainer authorization after this
proposal is reviewed and approved.

## Governing decision

The existing maintainer decision for SR-2 is:

  FIX_AND_DOCUMENT_PRE_1_0

The goal is NOT merely to change one save-path constructor from
Connection to Database.

Before implementation there must be a complete mapping in which public
OxiModError variants have one consistent meaning across the API.

The preferred design direction is FAILURE-CLASS SEMANTICS.

The original underlying driver/runtime/serialization error must remain
available through std::error::Error::source() wherever the current public
variant supports a source.

## Audit basis

Primary evidence:

- W1-F11
- W1-V07

Relevant adjacent evidence may be consulted when it demonstrates an existing
error mapping, but D5 must not reopen other CLOSED SR items.

The frozen audit is immutable and read-only.

## Core design question

Define exactly what each existing public OxiModError variant means.

The design must cover every current variant, not only Connection and Database.

At minimum inventory and define:

- Connection
- GlobalClientInit
- GlobalClientMissing
- Serialization
- Aggregation
- Index
- Validation
- Database
- Custom
- Query

If the current enum contains additional variants, include them.

Prefer preserving the existing public enum shape.

Do NOT add, remove, rename, or structurally change public variants in this
design unless the agent demonstrates that a coherent contract is impossible
without doing so. Any such proposal must be clearly isolated as an
approval-gated alternative, not assumed.

## Required semantic model

The proposal must define a deterministic precedence/classification policy.

It must answer cases such as:

- unreachable MongoDB server during save;
- unreachable server during find/update/delete/count/exists/query;
- unreachable server during index establishment;
- unreachable server during aggregation, if applicable;
- duplicate-key rejection during save;
- duplicate-key rejection during update;
- other write-command/server rejections;
- BSON serialization failure before/during a write;
- BSON deserialization failure through find_by_id;
- the same malformed BSON through query().first()/all();
- invalid typed-query configuration;
- model validation failure;
- hook/custom-user failure;
- global-client missing;
- global-client initialization failure;
- index-specification/server failure that is NOT a connectivity failure;
- aggregation-specific failure that is NOT a connectivity failure.

The same underlying failure class must not map differently merely because a
different OxiMod public method was executing.

## Failure-class preference

The preferred direction is approximately:

- connectivity / server-selection / transport failures -> Connection;
- BSON/Rust encoding or decoding failures -> Serialization;
- model rule failures -> Validation;
- typed-query configuration failures -> Query;
- global-client lifecycle failures -> GlobalClientInit / GlobalClientMissing;
- user-defined hook/domain failures -> Custom;
- index-specific non-connectivity failures -> Index;
- aggregation-specific non-connectivity failures -> Aggregation;
- other MongoDB operation/server/write/read failures -> Database.

This is a design hypothesis, NOT an implementation instruction.

The design agent must inspect the actual current MongoDB-driver error kinds and
all OxiMod call sites and determine whether this precedence is technically
sound and maintainable.

If a better coherent ordering is required, propose it and explain why.

## MongoDB driver classification

Inspect the actual mongodb crate version used by this repository.

Determine which mongodb::error::Error / ErrorKind forms can reliably identify:

- connectivity/server selection/network failures;
- BSON serialization failures;
- BSON deserialization failures;
- write failures such as duplicate key;
- command/server failures;
- other relevant categories.

Do not guess from memory.

Account for non-exhaustive enums or future-driver compatibility where
applicable.

The proposed classifier must have a conservative fallback.

Do not design classification around string matching if a stable typed route
exists.

Do not discard the original mongodb::error::Error.

## Operation-specific variants

The existing enum contains operation-domain variants such as Index and
Aggregation.

The design must explicitly define their interaction with failure classes.

For example, determine whether:

  network outage while creating an index

should classify as Connection rather than Index, while:

  MongoDB rejects an incompatible index specification

classifies as Index.

Likewise define the equivalent precedence for Aggregation.

The contract must be predictable from the variant documentation alone.

## Complete call-site inventory

Search the entire current workspace for every place that:

- constructs OxiModError directly;
- calls OxiModError constructor helpers;
- uses map_err into OxiModError;
- converts QueryError;
- wraps mongodb::error::Error;
- wraps bson serialization/deserialization errors;
- wraps hook/custom errors;
- creates GlobalClient errors;
- creates Index or Aggregation errors.

Produce a table containing at least:

1. source location;
2. public operation/path;
3. underlying source error type;
4. current OxiModError variant;
5. proposed OxiModError variant;
6. classification rule responsible for the proposed result;
7. whether behavior changes;
8. relevant regression needed.

Do not stop after the W1-V07 cases.

## Helper/API design

Determine the smallest internal architecture that can enforce the contract
consistently.

Evaluate, at minimum, a shared internal classifier such as:

  OxiModError::from_driver(...)

or an equivalent private/internal helper.

The proposal must specify:

- exact responsibility;
- inputs;
- whether operation context/domain is required;
- precedence ordering;
- fallback behavior;
- source preservation;
- whether helpers such as connection()/database()/serialization() remain;
- whether direct constructor use should be reduced to prevent recurrence.

Prefer one centralized policy over repeated ErrorKind matching at individual
call sites.

No implementation in D5.

## Source preservation

For every remapped driver error, preserve the original driver error through
Error::source().

The design must state whether consumers can still do, where applicable:

  error.source()
       .and_then(|source| source.downcast_ref::<mongodb::error::Error>())

Do not flatten driver errors into strings.

Do not discard server codes such as duplicate-key code 11000.

## Display/context policy

Separate:

- variant classification;
- human-readable operation context.

A failure becoming Connection or Serialization must still be allowed to carry
useful context such as:

  "failed to insert document"
  "failed to find document by _id"

The design must specify whether existing messages can largely be preserved
while variant selection changes.

Avoid making Display text the supported machine-classification API.

## Compatibility

Enumerate every measured/public mapping that would change from 0.3.0.

At minimum include the W1-V07 matrix.

Clearly state that exhaustive variant matching may observe changed behavior.

Provide proposed migration guidance.

No persisted BSON, validation timing, index lifecycle, or query semantics
should change merely as a consequence of SR-2.

## Regression design

Propose permanent internal regressions before implementation.

The regression matrix should cover the same FAILURE through multiple call
sites wherever practical.

At minimum design tests for:

1. duplicate key through save and update -> same intended class;
2. unreachable server through save and non-save operations -> same intended
   class;
3. malformed/deserialization failure through find_by_id and typed-query
   terminals -> same intended class;
4. client-side serialization failure -> intended Serialization class;
5. index server/spec conflict -> intended Index class;
6. index operation connectivity failure -> intended Connection class if that
   is the approved precedence;
7. GlobalClientMissing remains distinct;
8. Validation remains distinct;
9. Query configuration remains distinct;
10. source() preserves the original mongodb error and server code where
    relevant.

Use test isolation and short server-selection timeouts for unreachable-server
cases so the suite does not become unreasonably slow.

## External re-verification design

Design a source-hidden differential re-verification against crates.io
oximod = "=0.3.0".

Primary external target:

- W1-V07 error-classification cases.

Specify which baseline/candidate differences should be observed.

The eventual verifier must classify by the public OxiModError variant and
also inspect source() to prove the original driver error remains available.

## SR-13 adjacency

SR-13 is already CLOSED and must NOT be reopened.

Prior diagnosis noted that adding the offending document _id to
deserialization-error context could naturally touch the same error surface.

D5 may list that idea as an OPTIONAL adjacent follow-up decision, but it must
NOT silently include it in the SR-2 contract or implementation scope.

Default D5 scope excludes _id enrichment.

## Non-goals

D5 must NOT design or implement:

- new retry policies;
- automatic retry/backoff;
- a public duplicate-key convenience API unless separately proposed and
  explicitly approved;
- new MongoDB capabilities;
- transaction/session behavior;
- new query features;
- index lifecycle changes;
- validation changes;
- SR-13 behavior changes;
- any P-1 through P-12 item.

## Repository discipline

Read product source as needed.

Do NOT modify:

- README.md
- oximod/**
- oximod_core/**
- oximod_macros/**
- Cargo.toml
- Cargo.lock
- REMEDIATION_LEDGER.md
- prior remediation controls
- frozen audit

Do NOT commit.
Do NOT push.
Do NOT perform history operations.

The only permitted output file is:

  /tmp/oximod-d5-error-contract-proposal.md

## Required proposal

The final proposal must contain:

1. branch and HEAD observed;
2. complete current OxiModError enum inventory;
3. current documented meaning of each variant;
4. complete constructor/call-site inventory;
5. measured W1-V07 baseline mapping;
6. proposed semantic meaning of every variant;
7. deterministic classification precedence;
8. exact mongodb ErrorKind/category analysis;
9. proposed centralized internal classifier architecture;
10. treatment of Index and Aggregation versus Connection;
11. serialization/deserialization policy;
12. Database fallback policy;
13. source() preservation policy;
14. Display/context policy;
15. current -> proposed mapping table;
16. compatibility/breaking-behavior table;
17. migration guidance;
18. permanent regression matrix;
19. source-hidden external re-verification plan;
20. implementation files likely to change;
21. implementation files that should NOT need to change;
22. risks and ambiguous cases;
23. explicit confirmation that SR-13 and P-items remain excluded;
24. explicit yes/no recommendation on whether the complete failure-class
    contract is practical pre-1.0;
25. exact implementation authorization text the maintainer could use next.

Do not implement anything.

STOP after writing and reporting the proposal.
