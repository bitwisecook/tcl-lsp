# RUST_ISSUE_200: the recommended config routes `.irul`/`.iapp`/`.exp`/`.apl` through a single `name = "tcl"` language, so the server gets languageId `"tcl"` and defaults to tcl8.6, never selecting the expect/f5-irules/f5-iapps dialect

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Editor integrations |
| **Location** | `editors/helix/README.md:41-43 (same class in editors/emacs/README.md:55)` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

editors/helix/README.md:41-43 (same class in editors/emacs/README.md:55) — the recommended config routes `.irul`/`.iapp`/`.exp`/`.apl` through a single `name = "tcl"` language, so the server gets languageId `"tcl"` and defaults to tcl8.6, never selecting the expect/f5-irules/f5-iapps dialect.
`file-types = ["tcl", …, "irul","irule","iapp","apl","exp"]` with no `language-id` override → Expect commands flagged unknown, iRules analysis inert. Emacs `.apl → tcl-mode` has the same effect. Confidence: medium
