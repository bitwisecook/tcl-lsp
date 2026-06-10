# Differential-fuzzer findings corpus

This directory is a **data corpus, not importable package code.** Nothing
here is imported by the runtime — it is a set of regression fixtures the
fuzzer reads and writes.

Each finding is a pair:

- `seed_<n>.tcl` — the failing (or formerly-failing) Tcl script.
- `seed_<n>.json` — its metadata (backend mismatch, category, fixed/unfixed
  status, …).

## Produced by

`tooling/fuzzing/runner.py` writes a finding here whenever a differential
campaign turns up a mismatch (see `_save_finding`; gated on
`save_findings`).

## Consumed by

- `make test-fuzz-full` — the saved-findings regression sweep
  (`TestRegressions` in `tooling/fuzzing/tests/test_fuzz_differential.py`),
  opt-in via `FUZZ_FULL=1` so routine `make test-fuzz` stays fast.
- The `fuzz-findings` skill — query/triage/verify/mark findings.

## Packaging

This corpus is **excluded from the shipped wheel**
(`[tool.hatch.build.targets.wheel] exclude` in `pyproject.toml`). It is a
developer regression asset, not part of the installable library.
