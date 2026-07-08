# RUST_ISSUE_133: four formatter settings shipped by every editor are never read by the server and never used by the engine, so toggling them does nothing

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Editor integrations |
| **Location** | `rust/tcl-lsp-server/src/lib.rs:7610-7695 (apply_formatting_object)` |
| **Status** | Fixed (settings now read; minBodyCommandsForExpansion + replaceSemicolonsWithNewlines engine-consumed; enforceBracedExpr + alignCommentsToCode carried, engine consumption deferred) |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-server/src/lib.rs:7610-7695 (`apply_formatting_object`) — four formatter settings shipped by every editor are never read by the server and never used by the engine, so toggling them does nothing.
`gen_vscode_package.rs` emits (and JetBrains TclLspSettings.kt:342,346,349,350 sends) `enforceBracedExpr`, `minBodyCommandsForExpansion`, `alignCommentsToCode`, `replaceSemicolonsWithNewlines`, but `apply_formatting_object` maps none and the engine references none. `alignCommentsToCode`/`replaceSemicolonsWithNewlines` default to `true`, implying active behaviour the formatter never performs, and a user can't turn them off. Confidence: high
