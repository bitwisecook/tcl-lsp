# RUST_ISSUE_168: `str_replace`/`str_insert`/`str_case`/`str_trim` (and str_compare/str_match) convert via `String::from_utf8_lossy`, turning invalid-UTF-8 bytes into U+FFFD, while `str_index`/`str_range`/`str_length`/`str_reverse` preserve raw bytes. `string replace $bin 0 0 X` corrupts trailing binary bytes that `string index $bin` handles correctly — inconsistent within the same command family

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_string.rs:252,287,809,928` |
| **Status** | Fixed (GitHub issue #1309) |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

runtime/rust/src/cmd_string.rs:252,287,809,928 — `str_replace`/`str_insert`/`str_case`/`str_trim` (and str_compare/str_match) convert via `String::from_utf8_lossy`, turning invalid-UTF-8 bytes into U+FFFD, while `str_index`/`str_range`/`str_length`/`str_reverse` preserve raw bytes. `string replace $bin 0 0 X` corrupts trailing binary bytes that `string index $bin` handles correctly — inconsistent within the same command family. Confidence: medium

## Resolution (2026-08-07, GitHub issue #1309)

The named line numbers were stale by the time this was investigated: all of
`string index`/`range`/`replace`/`insert`/`toupper`/`trim`/`compare`/`match`
now route through the shared `tcl_cmd_core::string::dispatch_canon` (the local
`str_*` functions this issue names are dead code, superseded), so *every*
portable `string` subcommand shared the same corruption — not just the four
named here — via one root cause: `ValueOps::as_str` in
`runtime/rust/src/value_ops.rs` used `String::from_utf8_lossy` unconditionally.
Fixed at that single seam (`bytes_to_str`/`str_to_bytes`); see
[the KCS note](../docs/kcs/kcs-issue-runtime-string-subcommands-corrupt-binary-values.md)
for the mechanism and its one documented residual limitation.
