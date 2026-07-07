# RUST_ISSUE_052: `string last` panics (index out of bounds) on an empty haystack with an explicit index

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Support crates & regex |
| **Location** | `rust/tcl-cmd-core/src/string.rs:303-310` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cmd-core/src/string.rs:303-310 — `string last` panics (index out of bounds) on an empty haystack with an explicit index.
`string last a "" 0` crashes. With `hay=[]`, `needle=['a']`, `last_index="0"` → `last=0` (>= 0, early return skipped), clamp `min(hay.len().saturating_sub(1))` yields 0, then `last+1 (1) >= needle.len() (1)` passes, `hi=0`, loop slices `hay[0..1]` on a length-0 vector. `string first` guards this with `needle.len() <= hay.len()`; `last` does not. Confidence: high
