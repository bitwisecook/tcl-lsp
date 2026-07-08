# RUST_ISSUE_187: regex/glob hover build GFM tables with pipe tokens inside code spans, unescaped

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/hover.rs:1788` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-lsp-core/src/hover.rs:1788 — regex/glob hover build GFM tables with pipe tokens inside code spans, unescaped.
Hover inside `regexp {foo|bar}`: the `|` token emits `| \`|\` | Alternation |`; a raw pipe can break the row in strict GFM. Glob path identical at :1541. Confidence: medium (client-dependent)
