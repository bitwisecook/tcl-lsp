# RUST_ISSUE_053: `tcl-apl` is missing from `TCL_LANGUAGE_IDS`, so `.apl` (iApp APL) files receive no LSP features in VS Code

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Editor integrations |
| **Location** | `editors/vscode/src/languageIds.ts:23-38 (consumed at extension.ts:351)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

editors/vscode/src/languageIds.ts:23-38 (consumed at extension.ts:351) — `tcl-apl` is missing from `TCL_LANGUAGE_IDS`, so `.apl` (iApp APL) files receive no LSP features in VS Code.
package.json fully contributes the language (`{"id":"tcl-apl","extensions":[".apl"]}`), fires `onLanguage:tcl-apl`, the server maps `tcl-apl → f5-iapps`, and a test asserts it registers — but the `LanguageClient` `documentSelector` is built solely from `TCL_LANGUAGE_IDS`, which jumps from `"tcl-iapp"` straight to `"tcl-bigip"`. Opening an APL file activates the extension but attaches no client: zero diagnostics/completion/hover/semantic-tokens; `isTclLanguage("tcl-apl")===false` also disables the dialect status bar and save-triggered validation. `LANGUAGE_ID_DIALECTS` omits it too. Confidence: high
