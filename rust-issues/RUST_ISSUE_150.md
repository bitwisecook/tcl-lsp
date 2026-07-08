# RUST_ISSUE_150: the phi-merge trace skips version-0 (entry/live-in) incoming operands, so a value merged from a non-URI live-in path is still attributed to the URI getter

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Compiler front-end (segmenter/expr/subst) |
| **Location** | `rust/tcl-compiler/src/uri_split.rs:317` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/uri_split.rs:317 — the phi-merge trace skips version-0 (entry/live-in) incoming operands, so a value merged from a non-URI live-in path is still attributed to the URI getter.
`trace_to_uri_family`'s `if *inc_ver == 0 { continue; }` drops the caller/uninitialized reaching def, so `x = φ(x0_livein, x1_from_HTTP::uri)` traces to `HTTP::uri` and can fire a spurious IRULE3103. Same version-0-skip shape as the sccp/type_infer miscompile; here impact is a false-positive lint. Confidence: medium
