# RUST_ISSUE_050: `tcl pkg install` never resolves transitive dependencies: the MVS resolver is invoked with `provider: None`, so every direct requirement is treated as a leaf

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/pkg.rs:261` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-cli/src/commands/pkg.rs:261 — `tcl pkg install` never resolves transitive dependencies: the MVS resolver is invoked with `provider: None`, so every direct requirement is treated as a leaf.
Installing a package whose own `tclpkg.tcl` declares `require`s materialises only the direct package; its dependencies are silently absent from the lockfile and `lib/` (every lock entry's `requires` is empty, so `pkg list` also mislabels everything "direct"). `let input = ResolveInput { … provider: None, include_dev: !no_dev };` while `resolver.rs` exists specifically to walk `(name, version) → requires`. No disclosure is printed (unlike `update`/`outdated` which say "not yet wired"). Confidence: medium
