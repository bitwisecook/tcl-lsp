# RUST_ISSUE_028: `index_spec`/`parse_isize` parse integers DECIMAL-ONLY, though the doc-comment claims "the full TclGetIntForIndex grammar." Backs `lset`/`ledit` AND is imported by cmd_string.rs:1345 for `string index/range/first/last/replace/insert/toupper/tolower/totitle`; `str_repeat` uses it for count

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_list.rs:82,89,109` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

runtime/rust/src/cmd_list.rs:82,89,109 — `index_spec`/`parse_isize` parse integers DECIMAL-ONLY, though the doc-comment claims "the full TclGetIntForIndex grammar." Backs `lset`/`ledit` AND is imported by cmd_string.rs:1345 for `string index/range/first/last/replace/insert/toupper/tolower/totitle`; `str_repeat` uses it for count.
`string index abcdef 0x2` → `bad index "0x2"` (Tcl → `c`); `string index abcdef end-0x1` → error (Tcl → `e`); `lset x 0x1 Z` → error (Tcl → `a Z c`); `string repeat x 0x3` → error (Tcl → `xxx`). Stark because `lindex/lrange/lreplace/linsert` route to the radix-aware core and DO accept `0x1`. `while i < s.len() && s[i].is_ascii_digit()` Confidence: high
