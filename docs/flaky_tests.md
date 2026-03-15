# KCS: Flaky tests

## Summary

Tests that intermittently fail due to timing, test ordering, or shared
state — not due to real bugs.  These only reproduce in single-threaded
`pytest` runs; CI uses `pytest-xdist` (parallel) where they pass reliably.

## Known flaky tests

### `tests/test_analyser.py::TestProcAnalysis::test_proc_multiple_params`

- **Symptom**: assertion failure on proc parameter analysis
- **Root cause**: test ordering / shared global state from earlier tests
  (likely `configure_signatures` side effects leaking across tests)
- **Reproduces**: only in single-threaded full-suite runs (`pytest tests/`)
- **Passes**: in isolation (`pytest tests/test_analyser.py::TestProcAnalysis::test_proc_multiple_params`)
  and with `pytest-xdist` (`pytest -n auto`)

### `tests/test_async_diagnostics.py::TestSchedulerRapidUpdates::test_rapid_updates_only_last_published`

- **Symptom**: async timing assertion — the "last published" diagnostic
  doesn't match when the scheduler runs under heavy CPU load
- **Root cause**: timing-sensitive test that assumes rapid updates settle
  within a fixed window; single-threaded full-suite runs are slow enough
  to violate this assumption
- **Reproduces**: only in single-threaded full-suite runs under load
- **Passes**: in isolation and with `pytest-xdist`

## Mitigation

CI runs tests via `make prep-pr` which uses `pytest-xdist` for parallelism.
These flaky tests are not a gate concern.  If they start failing in CI,
investigate whether new shared state was introduced.
