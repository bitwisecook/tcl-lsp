# analyser-binding: pause handoff, 18 November 2026

Area: the binding walk and realm in `rust/tcl-compiler`:

- `command_binding`, `unit_scope`, `compilation_unit` and `inline_uplevel`;
- `dynamic_names`, `deferred_writes` and `word_subst`;
- `common_aot_plan`, `native_lowering` and `cfg_builder::upvar_info`;
- any unlisted tcl-compiler family;
- tcl-registry declarations whose effects stop the walk.

`origin/work` at pause: de2d53c8f plus this docs commit.

## (a) Landed

| Commit | What |
|---|---|
| b1da9a901 | A deleted builtin is absent in the diagnostic view (W123). |
| f57b576ea | A renamed or deleted builtin leaves the module's trust. The rename projection baseline is the registry's original table. |
| dad60e18e | `proc A::p …` with `::A` missing raises `unknown namespace` (tcl8.4–9.1, jim). Adds `closed_original_world`. |
| 3683b8089 | fp_obj_07 fixture creates its namespace (the tclsh evidence for dad60e18e). |
| 2c016d0c1 | A body the analysis cannot enter (no body indices, the registry says the command may run a Body) withdraws the dominating constants before it. |
| 6f29f2118 | `ExpressionMathBindings::for_module_statement` reads a procedure's declared-frame body. It removes the O101 rewrite of `proc add {a b} {return [expr {$a+$b}]}` to `return 7` from one `add 3 4` call. New 0 / fixed 0 on tcl-compiler, lsp-core proof groups new 0. |
| c0c673485 | `docs/work/test-scrub/declared-frame-switch.md`: the declared-frame switch's 65+1 measured regressions, grouped by fact lost, plus its 6 fixes. |
| de2d53c8f, 38af1ef0a | fmt only: files other sessions pushed unformatted. |

Also landed earlier: 682ef3264, a global_write_info gate fix with `is_absolute_bytes`.

## (b) In flight: everything is a patch, nothing half-done on work

All patches are under `docs/work/name-handling/handover/pause-2026-11-18/patches/`.

1. **`analyser-binding-operator-errors.patch`** (format-patch of local a4ddfbc8c, based on de2d53c8f). Apply with `git am`.
   - **Change:** `command_binding/source_walk/expression_walk.rs`. An arithmetic, bitwise, logical or list operator adds a Tcl Error alternative from the state its operands left, unless its operands are constants that evaluate. Comparisons (`== != < <= > >=`, `eq ne lt le gt ge`, the iRules string comparisons except `matches`/`matches_regex`) never raise. `&&`, `||` and `?:` raise on an operand whose truth is unproved.
   - **Witness:** new `it_2 var_escape_cfg::an_operator_that_may_raise_reaches_the_try_handler`. tclsh 8.6.18/9.0.4/9.1.0 with d=x: `/`, unary `-` and `&&` reach the handler; `==` does not.
   - **Root:** the walk never entered `try {expr {1/$d}} on error {…}` handlers. values' row `ir_upvar_in_try_handler_escapes` (with values' new fixture) passes with it.
   - **Measured:** full tcl-compiler vs de2d53c8f: 1891 → 1891.
     - New 1: `tcl-compiler::it_5 checks::commands::recursive_call_command_world::a_world_preserving_recursive_call_keeps_later_diagnostics`.
     - Fixed 1: `registry_invocation::source_structure::original_body_syntax::tests::nested_factorial_expressions_use_their_retained_source_compiler_vectors`.
     - The lsp-core proof groups were not run.
   - **Remaining:** find why the it_5 test regresses. Probably an error edge from an operator inside a recursive call widens the world after the call; check whether its `expr` operands are proved integers, so the error alternative should be removed by `native_numeric_result`. Fix or restate, re-measure to new 0, run the lsp-core proof groups, then land.
2. **`analyser-binding-declared-frame-switch.patch`** (park/declared-frame-switch 04b96763d).
   - **Change:** `retain_declared_frame_bodies` replaces every called procedure's own body with its declared-frame body.
   - **Measured:** fixes the `info exists` fold (`it_2 value_transfer_witnesses::try_and_errors::a_body_that_ends_in_an_error_may_have_failed_at_its_start`) but fails 66 new; see `docs/work/test-scrub/declared-frame-switch.md`.
   - **Blocked on:** dataflow (session …JkH4) making `declared_frame_walk` keep entry-independent facts (coordinator ruling). Do not land until re-measured to new 0.
3. **`analyser-binding-proc-rename-trust.patch`** (park/proc-rename-trust f65571a04). A failed shortcut, kept for reference. The item went to dataflow, with my review before they push.
   - **Real fix:** carry the walk's `proc_rebound_names` into the lattice `prepared_command_trust` reads.
   - **Witness:** `a_renamed_or_deleted_procedure_is_no_longer_trusted`, in the patch.
4. **`analyser-binding-namespace-unknown.patch`** (park/namespace-unknown, 2 commits).
   - **Contents:** namespace unknown query/reset keep the command world, and operands joined across activations.
   - **Status:** insufficient alone. A Local cell needs an activation identity, and the module body must bind through the declared frame. Parked behind item 2.

tk-oo's reproducer for the stale-global fault is their patch: `docs/work/name-handling/patches/tk-oo-eval-proved-constant-v2.patch`, pushed in 1272d0681 and not applied. I had not yet applied it.

## (c) Queue, in order

The coordinator's order: wrong answers first, then what blocks the most rows.

1. **Land the operator-errors patch** after fixing the it_5 regression. It's a wrong fact (handlers never walked) and unblocks values' try rows.
2. **The stale-global walk fault (tk-oo).**
   - **Witness:** after `proc f {} {global test; set test {proc M {} {}}}; set test {set x 1}; f; eval $test`, the produced name value of `::test` at the eval is still `set x 1`. Only visible with tk-oo's (A) patch applied; witness it_4 `an_eval_of_a_script_the_lattice_cannot_prove_stays_a_blocker`, second prefix.
   - **Suspicion:** the callee's write through the `global` link clears or stores `original_name_values` under a different key than the caller's, so `restore_execution_frame` (var_resolve/frame_restore.rs) copies the stale entry back.
   - **Unconfirmed lead:** with a plain optimiser probe (`puts $test`), nothing folds wrongly, so the stale data is only in `original_name_values`, not in constants.
   - tk-oo's (A) and 14 taint rows wait on it.
3. **`option_database_amended()`.** A bit on ModuleCommandBindings, ORed at joins, set by a reached `option add` / `option readfile` and by any opaque effect. I promised audit (…MNKJ) this field name before they write the ForcedEntrySpec selection. Registry declaration for `option add` / `option readfile` included.
4. **Core `after` completion routes** (promised to taint …HGxE6): `after ms script`, `after idle`, `after cancel/info` → Normal|Error, no synchronous script; `after ms` alone sleeps. Taint traced the uncalled-body over-abstention to the root completion: a known registry command with undeclared completion contributes `Unknown`, which withdraws the after-load world. **Not yet found:** which spec field the walk reads to produce that `Unknown` route. I promised tk-oo the exact field and a model command to copy before they declare the Tk constructors.
5. **Lenient `tcl` profile.** Give it release-agnostic stock bindings where every catalogued release agrees (`return`, `set`, `string length`). Blocks values' 15 rows under `lenient-tcl-logical-source-records`.
6. **Loop first-entry binding fact** for diagnostics' param_traits deep pass (`while {1} {eval $b}`). The joined head is correctly unknown, since tclsh shows a later pass can run a non-stock eval; the trait needs the first entry.
7. **values' two shimmer rows:** `walk-non-ascii-literal-name` (the proved walk ends at `set é 9`) and `walk-incr-amount-word-shape` (`incr value "$step"` loses Incr).
8. **The previous queue** (in the earlier ledger notes): dataflow O103 after-load fold; Pack Body word not entered; E002/E003 pack arity; server-e2e bare unknown head; variable_alias_templates computed upvar element; spectcl native_normal_store; embedded_substitutions; auto_path keying; `package require $computed`; head_blocked_only_by_glob_imports.

## (d) Waiting on / waited on

- **Waiting on:**
  - dataflow (…JkH4): the declared_frame_walk collector keeping entry-independent facts, with the same_command_world change I approved with conditions; and the rename-trust plumbing, my review before push.
  - audit (…MNKJ): the RetainedDispatch registry rows and the walk commit for my review.
  - tk-oo (…KsHn): Tk constructor completion routes, held until I send the field (queue item 4).
  - class-facts: nothing; 9598eda5 approved.
- **Others waiting on me:**
  - tk-oo: the stale-global fix.
  - taint: `after` routes.
  - audit: `option_database_amended()`.
  - values: the operator fix landing (they filed `ir_upvar_in_try_handler_escapes` under `walk-expression-operators-never-raise`).
  - diagnostics: the first-entry fact.
  - lsp-features: nothing; their rename-in-catch report was their probe offset (42 is the `rename` argument, the catch head is 54).
- **Reviews:**
  - Approved: live-inlining ccf5cb55 (ClausePlan), on condition the clause walk resolves with the admitted release's SurfaceQuery (decline when absent) and a test that a live 8.5 `try` gets no clause walk. They push after that; I haven't seen the final SHA.
  - Approved: class-facts 9598eda5.
  - Pending: taint/tk-oo `entry_after` (taint found the cause elsewhere; see queue item 4).
  - Pending: execution's namespace origin/current/code (they found the fixture fault, 1931aae8; the Jim half went to irules-registry-tooling).

## (e) Coordinator rulings not yet carried out

- **Required-oracle measurement:** adopt it (`bash scripts/dev/ensure-jim-oracle.sh`, `TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1`). Newly failing tests join the ledger as KEEP-ORACLE; report the count. Not done.
- **Fact shape for Tk callbacks** (my ruling, agreed with audit; the audit session writes registry and walk, I review):
  - completion stays the constructors' Normal|Error routes (tk-oo);
  - registration is a new `BodyExecutionSpec` variant `DeferredGlobalScriptUnlessForced(&'static ForcedEntrySpec)`: classic entry/spinbox, -textvariable present AND -validate ∉ {none}, plus `option_database_amended()`;
  - dispatch is a separate Knowledge-typed `RetainedDispatch` field on CommandSpec / SubCommand / CommandForm (None | TargetWindow(arg) | NamedVariable | AllPending; Unknown → AllPending), stated explicitly per row with a C notifier-audit test.
  - External X resources are a stated host assumption.

## (f) Last measured counts

- **de2d53c8f** (tcl-compiler, every target): 13157 run, 1891 failed, 6 skipped. Failing list in my scratchpad as Q0-fail.txt, not kept. Command:
  ```
  source scripts/dev/agent-build-env.sh
  CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0 \
    cargo nextest run -p tcl-compiler --no-fail-fast --status-level fail \
      --final-status-level none --failure-output never
  ```
- **lsp-core proof groups at 2c016d0c** (the last run with a baseline): `cargo nextest run -p tcl-lsp-core --lib --no-fail-fast -E 'test(rename) | test(refactor) | test(references) | test(definition)'`: 1029 run, 216 failed.
- **Ledger** (`docs/work/test-scrub/ledger/analyser-binding.tsv`): 418 rows; open 404, repaired 5, fixed 9. Fixed = passing at de2d53c8f; the 8 rows set today carry that note. tcl-codegen rows were not measured and stay open.

## (g) Things a successor would get wrong

- **Every called body is bound under a per-activation frame identity** (`::p@0/implementation…`), so `runs_in(procedure.body, declared_frame)` is false for every called body. Any rule keyed on "not in its declared frame" applies to all called bodies, not only joined ones.
- **Facts that hold for every caller seen are legitimate** (upvar `beyond_caller`, global writes, AOT formal proofs). Only body rewrites (O101 folds) must hold for every entry; that's why 6f29f2118 narrows to `ExpressionMathBindings`.
- **Fetch deep before rebasing.** `git fetch --depth=50` loses the merge base on a busy branch and rebases other sessions' commits; fetch with `--deepen=300` or more. The scratchpad push.sh used `--depth=300`.
- **The fmt check covers the whole workspace.** Other sessions push unformatted files; fix them in a separate fmt-only commit before pushing.
- **`ledger_status.py --check` takes a directory:** `python3 docs/work/test-scrub/tools/ledger_status.py --check docs/work/test-scrub/ledger`. values.tsv had `routed (owner)` statuses that fail it (told values).
- **Stash safety during measurement.** A measurement that stashes the tree is broken by any edit to the stashed file. Commit locally instead, or measure in order.
- **Avoid debug-info builds** (the disk filled at 18G of target/debug/incremental). Clean `/tmp/tcl-spectcl-cache-test-*` before deleting target/.
- **Product claims rest on tclsh, never the MCP tcl-lsp server**, which may be a stale build.
