# RUST_ISSUE_087: `find_element` skips only 2 bytes for a backslash escape, but C `TclFindElement` skips the full `TclParseBackslash` length, and a `\<newline>` escape *includes the following space/tab run* — so whitespace after a backslash-newline wrongly terminates the element

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Lexer & syntax tree |
| **Location** | `rust/tcl-syntax/src/list.rs:249-264` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-syntax/src/list.rs:249-264 — `find_element` skips only 2 bytes for a backslash escape, but C `TclFindElement` skips the full `TclParseBackslash` length, and a `\<newline>` escape *includes the following space/tab run* — so whitespace after a backslash-newline wrongly terminates the element.
`split_list("a\\\n b")` → `["a ", "b"]` (2 elements), while real Tcl `llength "a\\\n b"` yields 1 element `a b`. Affects all split variants and `llength`/`lindex` const-folds. Confidence: high
