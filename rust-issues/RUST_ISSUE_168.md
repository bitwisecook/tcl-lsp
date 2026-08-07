# RUST_ISSUE_168: `str_replace`/`str_insert`/`str_case`/`str_trim` (and str_compare/str_match) convert via `String::from_utf8_lossy`, turning invalid-UTF-8 bytes into U+FFFD, while `str_index`/`str_range`/`str_length`/`str_reverse` preserve raw bytes. `string replace $bin 0 0 X` corrupts trailing binary bytes that `string index $bin` handles correctly — inconsistent within the same command family

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_string.rs:252,287,809,928` |
| **Status** | Open — re-verified at the branch tip (2026-08-07) and promoted to **GitHub issue #1309**, which supersedes this description. Two corrections: (a) the root cause is a single site, `runtime/rust/src/value_ops.rs:69-71`, whose `ValueOps::as_str` is `String::from_utf8_lossy` — `cmd_string.rs:138` routes the portable subcommands through `tcl_cmd_core::string::dispatch_canon`, so every operand is lossy-converted before the subcommand sees it, and the local `str_replace`/`str_insert`/`str_case`/`str_trim` named below are no longer the path taken; (b) `string index` and `string range` do **not** preserve raw bytes as claimed here — `string index [binary format H* 41ff42] 1` yields `efbfbd` where `tclsh9.0` and `tclvm` both yield `ff`. `tcl-vm` is unaffected: `rust/tcl-vm/src/value_ops.rs:95-97` implements the same method losslessly. |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

runtime/rust/src/cmd_string.rs:252,287,809,928 — `str_replace`/`str_insert`/`str_case`/`str_trim` (and str_compare/str_match) convert via `String::from_utf8_lossy`, turning invalid-UTF-8 bytes into U+FFFD, while `str_index`/`str_range`/`str_length`/`str_reverse` preserve raw bytes. `string replace $bin 0 0 X` corrupts trailing binary bytes that `string index $bin` handles correctly — inconsistent within the same command family. Confidence: medium
