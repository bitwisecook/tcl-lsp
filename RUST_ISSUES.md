# RUST_ISSUES — residue of the origin/rust branch bug sweep

A deep review of the `origin/rust` branch (Rust workspace, editor integrations,
build tooling) was run on 2026-07-07 and raised 206 findings across the lexer and
syntax tree, the compiler front-end, lowering, the middle-end, the analyser, the
three execution backends, the command registry, the LSP server, the BIG-IP model
and F5 tooling, the CLIs, the editor integrations, and build tooling and CI.

Every finding was re-validated against the branch tip on **2026-08-07**. **203 of
206 are resolved** — fixed, or (in one case) closed as by-design — and both their
per-finding files and the `RUST_ISSUE_NNN` references that were scattered through
the source have been removed. Fixed defects belong in the git history, not in a
tracker or a comment.

The three that survived are on the GitHub issue list, where open work belongs.
One has since closed and two are partly done:

| # | GitHub | What is left |
|---|---|---|
| [008](rust-issues/RUST_ISSUE_008.md) | [#1311](https://github.com/bitwisecook/tcl-lsp/issues/1311) | **Narrowed.** `yield` now crosses `try` (body/handler/`finally`), a bare `apply`, and a value-consumed `lmap`/`foreach`. Left: a `[yieldto …]` in a command-substitution *argument* slot (`coroutine.test` 7.3/12.1, lowers to runtime `subst_word`), not yet reduced to a standalone repro. |
| [014](rust-issues/RUST_ISSUE_014.md) | [#1313](https://github.com/bitwisecook/tcl-lsp/issues/1313) | **Partly done.** The fuzzer now pairs any two backends (`runtime/rust` ↔ `tclsh` added) and compares error text behind `--compare-error-text`. Left: characterising the `tcl-vm` ↔ `runtime/rust` pair (plumbing exists, no real campaign), and the real linked WASM runtime and eBPF arms. |
| ~~[168](rust-issues/RUST_ISSUE_168.md)~~ | [#1309](https://github.com/bitwisecook/tcl-lsp/issues/1309) | ✅ **Fixed** — `ValueOps::as_str`'s lossy conversion no longer corrupts non-UTF-8 bytes across the portable `string` surface. |

Their files are kept under [`rust-issues/`](rust-issues/) for the detail the
GitHub issues summarise; each carries a re-validation note and its issue number.
When all three close, this file and that directory go with them.

## Notes worth keeping

Three conclusions from the re-validation are not obvious from the diff, and were
each wrong in the tracker before it:

- **`lsort -command` is not a coroutine barrier.** It was listed as one under
  008. C Tcl refuses a `yield` inside an `lsort` comparator too, so the VM
  matches — there is nothing to fix.
- **The eBPF backend's small typed subset is a boundary, not a bug** (145). It
  rejects `set` / `incr` / bare `expr` with a specific diagnostic naming the
  typed replacement. Every rejection is explicit; there is no silent divergence.
- **`runtime/rust`'s binary-string corruption is one site, not four** (168). The
  four `cmd_string.rs` functions originally named are no longer even on the path;
  `ValueOps::as_str` is, and `string index` / `string range` are affected too,
  contrary to what the finding claimed.
