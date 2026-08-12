# KCS: variable case-mismatch suggestions

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
   the same CFG function scope -- not variables that are merely read. The
   defined set is collected by walking every block's statements and taking the
   assignment / `incr` targets plus the explicit defs on call statements.
2. A case-insensitive twin always beats an edit-distance match. W210 falls
   back to a length-scaled edit-distance suggestion when no twin exists (and
   never suggests the variable as its own correction); W211 and W220 offer the
   case-mismatch suggestion only.
3. The suggestion is purely informational; no `CodeFix` is attached to any of
   W210 / W211 / W220, because the analyser cannot determine which spelling
   the user intended.
4. Commands whose spec sets `safe_on_uninit` (e.g. `lappend`, `append`, `incr`
   on 8.5+) suppress W210 for the variable they define, even when a case
   mismatch exists. The remaining W211/W220 for the *other* spelling still
   carry the suggestion. The field is a `DialectSet`, so the suppression is
   per-dialect rather than a plain boolean.

## File-path anchors

- `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` -- `find_case_mismatch`, `undefined_var_suggestion`, and the W210 / W211 / W220 emitters
- `rust/tcl-compiler/src/analyser/diagnostics/helpers.rs` -- `collect_defined_vars`
- `rust/tcl-compiler/src/analyser/diagnostics.rs` -- where the defined-variable set is built per function unit
- `rust/tcl-compiler/src/text.rs` -- `suggest_similar` / `scaled_max_distance_strict`, the edit-distance fallback
- `rust/tcl-registry/src/spec.rs` -- the `safe_on_uninit` field on `CommandSpec` and `SubCommand`

## Test anchors

- `rust/tcl-compiler/src/analyser/diagnostics/tests.rs` -- the `did_you_mean_variables` module

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [LSP diagnostics publication](../../../docs/design/contracts/lsp-diagnostics-publication.md)
- [compiler diagnostics integration](../../../docs/design/compiler/diagnostics-integration.md)
