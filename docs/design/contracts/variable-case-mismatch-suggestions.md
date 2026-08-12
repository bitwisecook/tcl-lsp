# Variable case-mismatch suggestions

## Symptom

A user sets a variable with one casing (e.g. `set myList {a b c}`) and later
references it with different casing (e.g. `lappend mylist e`). Tcl variable
names are case-sensitive, so these are distinct variables. The diagnostics
(W210, W211, W220) fire correctly but the messages do not explain *why* the
variable appears unused or undefined.

## Operational context

When W210 ("read before set"), W211 ("set but never used"), or W220 ("dead
store") is emitted for a variable whose name differs only in case from another
variable defined in the same CFG scope, the diagnostic message is augmented
with `; did you mean 'X'?` where X is the case-insensitive match.

The suggestion is deterministic: when multiple candidates exist, the
lexicographically smallest name is chosen.

## Decision rules / contracts

1. Case-mismatch detection only considers variables *defined* (assigned) in
   the same CFG function scope -- not variables that are merely read.
2. The suggestion is purely informational; no `CodeFix` is attached because
   the analyser cannot determine which spelling the user intended.
3. Commands with `safe_on_uninit` (e.g. `lappend`, `append`, `incr` on 8.5+)
   suppress W210 for the variable they define, even when a case mismatch
   exists. The remaining W211/W220 for the *other* spelling still carry the
   suggestion.

## File-path anchors

- `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` — `find_case_mismatch`
  and the two sites that append the `; did you mean 'X'?` suffix.
- `rust/tcl-registry/src/spec.rs` — the `safe_on_uninit` field on `CommandSpec`.
- `rust/tcl-compiler/src/text.rs` — the shared suggestion-ranking cores
  (`rank_suggestions`), which every did-you-mean suffix goes through
  ([shared-utility-contracts-rust.md](shared-utility-contracts-rust.md)).

## Discoverability

- [Design doc index](../README.md)
- [LSP diagnostics publication](lsp-diagnostics-publication.md)
- [compiler diagnostics integration](../compiler/diagnostics-integration.md)
