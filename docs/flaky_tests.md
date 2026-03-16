# KCS: Flaky tests

## Summary

No known flaky tests.  Previously-documented issues have been fixed:

1. **`test_proc_multiple_params`** — The `conftest.py` autouse fixture now
   clears `SIGNATURES` before calling `configure_signatures`, bypassing the
   early-return optimisation that allowed in-place mutations to persist
   across tests.

2. **`test_rapid_updates_only_last_published`** — Replaced the fixed
   `asyncio.sleep(0.5)` with a polling helper (`_wait_for`) that retries
   for up to 5 s, eliminating timing sensitivity under CPU load.
