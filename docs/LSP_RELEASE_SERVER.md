# Native LSP server release status

- #1657: The edit-ticket reservation already accounts for cancelled waiters. The request admission pool also now releases its permit while waiting for the edit barrier, so a stalled edit cannot consume all request slots and block an unrelated request. A deterministic transport regression covers the real barrier and checks that waiters resume once the edit completes. This limits the effect of an edit stall; it does not establish the cause of every reported wedge.
- #2021: The native server already has an exit watchdog for stdin EOF and the `exit` notification, with process-level regressions for both paths during a workspace scan. Verification is pending in this checkout.
- #2295: The first workspace-symbol query already waits on startup scan readiness with separate bounded budgets for scan start and completion. The existing end-to-end test checks that the first query finds the on-disk fixture. This intentionally permits a stale result if the scan exceeds its time budget; verification is pending in this checkout.

Focused #1657 test attempted: Cargo could not download `thin-vec 0.2.21` because the configured proxy was unreachable. The alternate preinstalled Cargo cache also lacks the locked `sha2 0.11.1` version.
