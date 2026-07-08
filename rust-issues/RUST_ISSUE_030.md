# RUST_ISSUE_030: `normalise` runs `line.trim_end()` on every line and collapses trailing blank lines before comparison, so the differential harness cannot see trailing-whitespace or trailing-blank-line divergences

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Bytecode VM |
| **Location** | `rust/tcl-fuzz/src/harness.rs:108` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-fuzz/src/harness.rs:108 — `normalise` runs `line.trim_end()` on every line and collapses trailing blank lines before comparison, so the differential harness cannot see trailing-whitespace or trailing-blank-line divergences.
A VM bug in `format "%-5s"` / `string repeat " "` padding (tclsh `"x    \n"`, VM `"x\n"`) normalises to the same string on both sides → Verdict::Match, a false negative for exactly the class of output bugs the harness exists to catch. `for line in s.lines() { out.push_str(line.trim_end()); out.push('\n'); } while out.ends_with("\n\n") { out.pop(); }` Confidence: high
