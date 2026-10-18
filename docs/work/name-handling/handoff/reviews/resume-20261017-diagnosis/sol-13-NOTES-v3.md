# Sol-13 v3: layer-a-v3.patch (response to Luna-13 REVIEW-v2, blocker B1)

**Patch.** `layer-a-v3.patch`: sha256 `ed83a1de0963a4ebb8b107da1ba1d2a6e9bcc810f7400ba640728b2091421340`, 458 lines, made with `diff -ruN a b` against `git show HEAD:`. The six touched rust sources are unchanged from d0cd9d73b through c53603124.

**Checks.** All static; nothing was compiled or run.

- `git apply --check` passes on both the worktree and the index.
- rustfmt (edition 2024) is clean on all six files.
- The patch adds no `#[allow]`.

**Unchanged from v2.** Everything listed in NOTES-v2.md still holds: the registry `select_for_original_facts`, the shared seal premises (`closed_pattern_transfer`), the `reconfigured` flag on mapping and late lookup, and the tests. This note covers only the delta.

## Delta: the B1 fix (Luna's option b)

### State

`NamespaceResolutionProjection` replaces v2's single `ensemble_reconfigured` bool with `ensembles: EnsembleManagement { reconfigured, created }`. It is a separate small struct: with four bools in one struct, pedantic `struct_excessive_bools` would fire, and no new `#[allow]` may be added.

- **Setting the bits.** `record` sets `created` on `Ensemble { operation: Create }`. It sets `reconfigured` on Configure, Other and unresolved operations, exactly as v2 did. `Exists` sets neither.
- **Join.** Both bits are joined monotonically by OR.
- **Observation stamp (N1).** `NonBindingObservationStamp` now carries `namespace_ensembles`.

### Where `created` is consumed

Only on routes that compiled bodies take. Root Direct dispatch never reaches either one, so root precision is unchanged.

1. **`original_named_selection_is_closed`.** It returns false once `created` is set. The selection then falls back to `select_for_facts`, which is the HEAD behaviour: `Unknown` leads to the dispatch residual.
2. **`successful_handler_transfer`.** It returns None when the handler needs a stock worker (`Required` or `RequiredPath`) and `created` is set.
   - This is the bounded-handler route for compiled-unknown members, for example `string repeat` via `EnsembleLeaf`.
   - Without this second gate, c9 stays proved: `string repeat` in a body falls back to that route, which proves the stock worker because the authored mapping never sees the create.
   - The fallback is again the HEAD residual (`mark_opaque_binding_mutation`).

`reconfigured` keeps v2's consumers: `original_ensemble_mapping_holds` and `with_late_lookup`.

### Tests

`source_body_namespace_statements_outside_the_closed_premises_stay_open` gains two cases, both asserting that `incr x` has no proved target:

- **c6:** `rename auto_import savedPrelude`, then `namespace eval ::fake {namespace ensemble create -command ::namespace -map {… export ::hijack}}`, then a body `namespace export h`. It is refused by gate 1.
- **c9:** `namespace eval ::string {proc repeat …; namespace export repeat; namespace ensemble create}`, then `namespace eval ::b {string repeat a 2}`. It is refused by gate 2. The export in the `::string` body runs before the create, so it is unaffected.

c7 (the `-subcommands` form) goes through the same `Create` operation, so it is covered too, but no test was added for it.

## Precision cost

After any `namespace ensemble create`, these return to HEAD behaviour, a global residual, but only inside compiled bodies:

- compiled named namespace statements;
- compiled-unknown stock ensemble members that need a worker (`binary`, `info`, `string`, `file` and `array` named members).

Inline members such as `string length`, and everything at the root, are unaffected.

## Follow-up (Luna's later refinement)

Add a Registry accessor on `NamespaceTransition::Ensemble` that names the affected ensemble command:

- for `configure`, the command word;
- for `create`, the `-command` value, or else the current namespace's name.

Then scope both bits to targets that may be Registry-backed. That recovers v2's configure precision cost (N2) and v3's create precision cost.

## Gaps still tracked

These are pre-existing and unchanged:

- Root Direct imports and forgets do not get the seal premises.
- Root Direct dispatch and inline members ignore a replaced or reconfigured ensemble (e9, and c3 at the root).
- A failed `apply_import` passes silently.

## Withheld under the sound rule

Unchanged from v2, with fixtures not edited:

- `source_c_import_follows_the_original_command_object`;
- the 404 family;
- workspace tests 022, 024, 028 and 029 wherever they import inside a body without `rename auto_import`.
