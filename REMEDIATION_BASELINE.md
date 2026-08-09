# OxiMod Audit Remediation Baseline

## Audited release

OxiMod 0.3.0 was black-box audited from crates.io.

Published crates:

- oximod 0.3.0
- oximod_core 0.3.0
- oximod_macros 0.3.0

All three published crates contain `.cargo_vcs_info.json` identifying:

51cdf57dce8b7a9615008a2e325e11894e64cd39

as their source Git commit.

## Remediation baseline

Branch:

audit-remediation

Baseline commit:

51cdf57dce8b7a9615008a2e325e11894e64cd39

The remediation campaign therefore begins from the exact Git commit recorded
by all three crates.io 0.3.0 packages.

## Audit record

The completed black-box audit is preserved separately at:

/home/arshia/Code/oximod-blackbox-audit-final

The closed audit archive is:

/home/arshia/Code/oximod-blackbox-audit-closed-2026-08-08.tar.gz

The audit evidence must remain read-only during remediation.

## Separation rule

- Audit evidence: immutable.
- Remediation source: source-aware.
- Final regression verification: source-hidden again.
