# RUST_ISSUE_054: no `[language_servers.tcl-lsp.language_ids]` mapping, so the server receives languageIds it doesn't recognise for iRules/iApps/APL and silently analyses them as the default `tcl8.6`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Editor integrations |
| **Location** | `editors/zed/extension.toml:31-33` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

editors/zed/extension.toml:31-33 — no `[language_servers.tcl-lsp.language_ids]` mapping, so the server receives languageIds it doesn't recognise for iRules/iApps/APL and silently analyses them as the default `tcl8.6`.
Zed declares `languages = ["Tcl","iRules","iApps","APL","TMSH","Expect"]` but never bridges those names to the server's expected ids. `dialect_from_language_id` (rust/tcl-lsp-server/src/lib.rs:2003-2024) only accepts `tcl-irule`/`f5-irules` etc., never `irules`/`iRules`. Zed derives the LSP id from the lowercased language name, so a `.irul` buffer arrives as `"irules"`; no match, no bigip basename, id≠`"tcl"` skips source detection → `default_dialect` = tcl8.6. iRules-specific diagnostics/events/completions don't activate in Zed. Confidence: medium
