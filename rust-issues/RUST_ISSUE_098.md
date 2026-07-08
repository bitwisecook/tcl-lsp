# RUST_ISSUE_098: the diagnostics publish re-checks currency under the `documents` lock but delivers *after* dropping it, so a concurrent `didClose` interleaves and stale diagnostics are re-published (and re-cached) for a closed document

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lsp-server/src/lib.rs:1009-1028` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-server/src/lib.rs:1009-1028 — the diagnostics publish re-checks currency under the `documents` lock but delivers *after* dropping it, so a concurrent `didClose` interleaves and stale diagnostics are re-published (and re-cached) for a closed document.
Worker passes `doc.revision == delivery.revision` and drops `docs` at the block end; `did_close` then removes the doc, publishes the clearing empty set (lib.rs:5027) and removes the pull-cache entry (lib.rs:5033); the worker resumes with `delivery.cache_and_deliver(diags).await` — re-inserting a `PullDiagEntry` for the closed URI and pushing squiggles after the "last word" empty publish, which then stick. Confidence: high
