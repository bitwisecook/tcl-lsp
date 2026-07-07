# RUST_ISSUE_168: `str_replace`/`str_insert`/`str_case`/`str_trim` (and str_compare/str_match) convert via `String::from_utf8_lossy`, turning invalid-UTF-8 bytes into U+FFFD, while `str_index`/`str_range`/`str_length`/`str_reverse` preserve raw bytes. `string replace $bin 0 0 X` corrupts trailing binary bytes that `string index $bin` handles correctly — inconsistent within the same command family

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_string.rs:252,287,809,928` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

runtime/rust/src/cmd_string.rs:252,287,809,928 — `str_replace`/`str_insert`/`str_case`/`str_trim` (and str_compare/str_match) convert via `String::from_utf8_lossy`, turning invalid-UTF-8 bytes into U+FFFD, while `str_index`/`str_range`/`str_length`/`str_reverse` preserve raw bytes. `string replace $bin 0 0 X` corrupts trailing binary bytes that `string index $bin` handles correctly — inconsistent within the same command family. Confidence: medium
