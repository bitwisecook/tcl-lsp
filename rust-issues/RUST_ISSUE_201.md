# RUST_ISSUE_201: declared as an override ("When null, enabled") but no consumer exists: neither extension nor server reads `features.progress` (server emits work-done progress unconditionally). Setting it `false` does nothing

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Editor integrations |
| **Location** | `editors/vscode/package.json:3430 (tclLsp.features.progress; also TclLspSettings.kt:72,329, Zed README)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

editors/vscode/package.json:3430 (`tclLsp.features.progress`; also TclLspSettings.kt:72,329, Zed README) — declared as an override ("When null, enabled") but no consumer exists: neither extension nor server reads `features.progress` (server emits work-done progress unconditionally). Setting it `false` does nothing. Confidence: high
