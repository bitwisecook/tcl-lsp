# Variable case-mismatch suggestions

Why a variable diagnostic sometimes ends with `; did you mean 'X'?`. Tcl
variable names are case-sensitive, so `set myList {a b c}` and a later
`lappend mylist e` are two distinct variables; the suffix names the near-miss
spelling that explains why one of them looks unused or undefined.

When W210 ("read before set"), W211 ("set but never used"), or W220 ("dead
store") is emitted for a variable whose name differs only in case from another
variable defined in the same CFG scope, the diagnostic message is augmented
with `; did you mean 'X'?` where X is the case-insensitive match.

The suggestion is deterministic: `find_case_mismatch` collects every
*other* defined name whose lower-cased form equals the subject's, sorts them,
and returns the lexicographically smallest.

## Decision rules / contracts

1. Case-mismatch detection only considers variables *defined* (assigned) in
   the same CFG function scope -- not variables that are merely read.
2. The suggestion is purely informational; no `CodeFix` is attached because
   the analyser cannot determine which spelling the user intended.
3. Commands with `safe_on_uninit` (e.g. `lappend`, `append`, `incr` on 8.5+)
   suppress W210 for the variable they define, even when a case mismatch
   exists. The remaining W211/W220 for the *other* spelling still carry the
   suggestion. `safe_on_uninit` is `Option<DialectSet>` on `CommandSpec`, so
   the exemption is dialect-gated rather than universal.
4. **W210 has a second tier the other two do not.** `undefined_var_suggestion`
   tries `find_case_mismatch` first — a case twin wins at any edit distance —
   and only when that misses falls back to `text::suggest_similar` over the
   other defined names, bounded by `text::scaled_max_distance_strict` so a
   short typo cannot fish an unrelated short name. The read variable is
   filtered out of its own candidate set, since a variable assigned *later*
   in the function (`puts $x; set x 1`) is in `defined_vars` and must never be
   suggested as its own correction. W211 and W220 call `find_case_mismatch`
   directly and have no edit-distance tier.

## File-path anchors

- `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` —
  `find_case_mismatch`, `undefined_var_suggestion`, and the three sites that
  append the `; did you mean 'X'?` suffix (W210, W211, W220).
- `rust/tcl-registry/src/spec.rs` — the `safe_on_uninit` field on `CommandSpec`.
- `rust/tcl-compiler/src/text.rs` — the shared suggestion cores:
  `suggest_similar` (edit-distance filter) feeding `rank_suggestions`
  (ordering), plus `scaled_max_distance` / `scaled_max_distance_strict` for
  the length-scaled budget. Every edit-distance did-you-mean suffix goes
  through them
  ([shared-utility-contracts-rust.md](shared-utility-contracts-rust.md)).

## Discoverability

- [Design doc index](../README.md)
- [LSP diagnostics publication](lsp-diagnostics-publication.md)
- [compiler diagnostics integration](../compiler/diagnostics-integration.md)
