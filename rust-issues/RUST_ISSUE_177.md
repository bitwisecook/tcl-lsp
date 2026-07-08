# RUST_ISSUE_177: `[file join $dir a b]` tails are joined with a space instead of `/`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/package_resolver.rs:203-207` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-lsp-core/src/package_resolver.rs:203-207 — `[file join $dir a b]` tails are joined with a space instead of `/`.
For `package ifneeded p 1.0 [list source [file join $dir src impl.tcl]]` the candidate becomes `pkg_dir/"src impl.tcl"`, the `exists` probe fails, and the parser falls back to "every `*.tcl` in the dir" or drops the entry. `words[3..].iter().map(...).join(" ")`. Confidence: medium
