# RUST_ISSUE_132: `[[:upper:]]`/`[[:lower:]]` under `-nocase` fold to `alnum`, not `alpha`, so they wrongly match digits

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Support crates & regex |
| **Location** | `rust/tcl-regex/src/parser.rs:1552-1554` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-regex/src/parser.rs:1552-1554 — `[[:upper:]]`/`[[:lower:]]` under `-nocase` fold to `alnum`, not `alpha`, so they wrongly match digits.
C Tcl's `cclass()` remaps `CC_UPPER`/`CC_LOWER` to `CC_ALPHA` (letters only) when case-insensitive; `ClassKind::Alnum` also matches digits. So `regexp -nocase {[[:upper:]]} 5` matches here but not in tclsh, and `regexp -nocase {^[[:lower:]]+$} "abc9"` wrongly matches. Not covered by the reg.test corpus (no `-nocase` POSIX-class case), so the crate's tests pass. `if set.nocase && matches!(k, ClassKind::Lower | ClassKind::Upper) { k = ClassKind::Alnum; }` Confidence: high
