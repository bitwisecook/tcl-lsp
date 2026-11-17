# Hand-over: compiler dataflow and optimiser

Written 17 November 2026 by the outgoing session. `work` was at `da039ed1` when this was written. Nothing of this area is held outside the repository except what the two patch files below carry.

The long record of this area is the section "Compiler dataflow and optimiser half, 15 November 2026" in `docs/work/name-handling/current-state.md`. It lists every fix with its tclsh evidence and every open cause with its smallest fixture. This file is the short form and the order of work; read that section for the detail behind each queue item.

## Scope

Crate `tcl-compiler`. This area owns:

- Lowering (`src/lowering/`), CFG construction (`src/cfg_builder/`), SSA (`src/ssa/`), SCCP (`src/sccp/`), the constant folder (`src/tcl_expr_eval/`), the optimiser (`src/optimiser/`), interprocedural transfer (`src/interprocedural/`), object types (`src/object_types/`), `src/compilation_unit/`, GVN and the inliners.
- Integration tests `tests/it_3/optimiser.rs`, `tests/it_5/optimiser_coverage.rs`, `pipeline_coverage`, `optimiser_depth`, and the optimiser rows of `tcl-cli` `value_transfers_cli` (`opt_*`, `o103_*`).

It does not own, although the same suite runs them:

- The execution walk and binding (`src/command_binding/`, `src/registry_invocation/`): `analyser-binding`. Most failures in this area's modules wait on it.
- Analyser diagnostics (`src/analyser/diagnostics/`): `diagnostics`.
- Taint, `irules_checks`, `uri_split`: `taint`.
- `value_transfer_witnesses`, `var_escape`, `shimmer`, `type_infer`, `native_integer_proof`, `upvar_info`, `global_write_info`, `unit_scope`, `intervals`: `values`.
- Class and object facts gathered by the walk: `class-facts`. `object_types` reads them.
- Emission (`tcl-codegen`, `tcl-vm`): `execution`.

## How to measure

From `rust/`:

```bash
source ../scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0
cargo nextest run -p tcl-compiler -p tcl-registry --no-fail-fast \
  --status-level fail --final-status-level none --failure-output final 2>&1 \
  | cut -c1-900 > LOG
grep '^ *FAIL \[' LOG | sed 's/^ *FAIL \[[^]]*\] *//' | sed 's/^([^)]*) *//' | sort -u > LIST
```

The run takes about 14 minutes once built and 25 to 30 minutes with the rebuild that follows a rebase. Nothing is excluded; three tests are slow but finish. Compare `LIST` with the baseline using `comm -13` (new) and `comm -23` (fixed). Since the file split a test's path gained a module, so compare the baseline below by the last path segment (`awk -F'::' '{print $NF}'`), or take a fresh baseline first, which is the better choice.

Edit loop for one module: `cargo nextest run -p tcl-compiler --lib -E 'test(/name_fragment/)'`. For an integration group: `--test it_3 -E 'test(/…/)'`.

Gates before a push: `cargo fmt --all -- --check`; `cargo clippy -p tcl-compiler --all-targets -- -D warnings -A clippy::assert_is_empty`; `make xtask-gates` from the repository root (39 gates, about five minutes). A change to `optimiser/propagation/` or any value-transfer site makes `docs/generated/value-transfers.md` stale: run `cargo xtask value-transfers` and commit the result.

To list panic messages for a group of failures from a log:

```python
import re, sys
t = open(sys.argv[1], errors='replace').read()
for m in re.finditer(r"thread '([^']*)' \(\d+\) panicked at ([^\n]*):\n([^\n]*)(?:\n([^\n]*))?", t):
    if sys.argv[2] in m.group(1):
        print(m.group(1)[-70:], '|', m.group(2)[-32:], '|', ' '.join(x[:140].strip() for x in m.groups()[2:] if x))
```

## Current numbers

Last full run: 16 November 2026, on `836282e8` plus the commit that became `17abfa4f` (pre-restructure layout). `tcl-compiler` and `tcl-registry` together: 15,578 run, 2,236 failing, 6 skipped. One of the 2,236 is in `tcl-registry`. No full run was completed on the restructured tree: the pause arrived while the first one was building.

By module in that list, this area's own: `optimiser` 191, `cfg_builder` 85, `lowering` 72, `sccp` 46, `ssa` 44, `interprocedural` 38, `optimiser_coverage` 25, `compilation_unit` 24, `inline_uplevel` 16, `object_types` 15, `pipeline_coverage` 10, `optimiser_depth` 10, `gvn` 7. The largest groups overall belong to other areas (`analyser` 520, `command_binding` 283).

## Baseline list

`docs/work/name-handling/lists/dataflow-optimiser-failing-2026-11-16.txt`: the 2,236 failing tests of that run, one per line, as `binary path::to::test`. It is the whole shared suite, not only this area, so that a new failure anywhere shows up. Paths are pre-restructure.

## Queue

In order. The first two are small and ready; the rest wait on the walk or need tracing.

1. **A call to a user procedure flags the whole module.** Reported by `class-facts`. `Lowerer::definer_invocation_may_supply_body` (`src/lowering/declarations/definer_calls.rs`) answers true when the head has no registry candidate, so `helper 1` or `C create c` sets `oo_evidence.unretained_executable_roots`, and `command_binding` then treats the module's bindings as opaque (`procedure_inventory_complete`, `discarded_opaque_binding_mutation` in `src/command_binding/module_bindings/`). On `origin/rust` a head not in the registry answers false. My reading, untested: this is case 1, and only `advice.unknown_residual` (the binding's execution is unknown) should answer true; a known binding with no registry candidate is a user procedure whose body is retained. The patch is below. Check first what `candidates` is empty for when the walk did not reach the command, since that must stay true. Tests to watch: `lowering::tests` (72 failing), `command_binding/tests/evaluated_bodies.rs` and `lowering/original_script_purpose_tests.rs`, which assert the flag is set for their shapes.
2. **Inlined body inside a static `eval` and inside an `lmap` body.** `proc caller {n} {eval {vdouble $n}}` is not restored to a structured statement by `17abfa4f`, which only drops the control-plan early return in `Lowerer::try_dispatch_structured_hook` (`src/lowering/commands/structured_hooks.rs`) when there is no native entry. Tests: `tcl-spectcl` `codegen_stamps::an_inlined_body_inside_a_static_eval_keeps_its_value` and `an_inlined_body_in_an_lmap_body_gives_the_list_each_iterations_value`. Not traced further.
3. **A fold leaves its dead store in place within one pass.** `set a 1; set b [expr {$a + 1}]; puts $b` gives `set a 1` and `puts 2`; the test expects `puts 2` alone. Optimiser pass ordering, `src/optimiser/`.
4. **O100 into an expression operand never fires.** `substitute_expr_constants_for_execution` (`src/optimiser/helpers/expr_simplify/constant_folding.rs`) is called with no operand receipts and never changes the text. The working folds go through `PassContext::eval_expression_at` (`src/optimiser/mod.rs`) instead.
5. **`tcl-cli` `value_transfers_cli`.** `explore_sccp_prints_the_selection`, `explore_sccp_prints_the_route_tally`, `o103_summary_path_folds_a_computed_return` under `tcl8.4`, `the_seven_summary_witnesses`: the explorer prints `sccp.values`, which is overdefined where the test expects a value. Under the ruling below the lattice stays strict, so either the explorer prints the semantic facts beside it or the fixtures move; decide per test with tclsh.
6. **No procedure has a transfer summary** (blocks #2408 and #1796). In a release build `SourceSummaryContext::for_module` is `None`, so `ModuleProcedures::summarise` does nothing (`src/interprocedural/`). Traced to there only.
7. **Three open rows of the fused-expression table** (`sccp::tests::a_fused_expression_has_a_value_and_maybe_an_execution_constant`): the semantic facts hold no value for `expr {"x"}` under `tcl8.4`, `expr {"08"}` under `tcl8.6` and `[string length abc] * 2` under `tcl9.0`.
8. **`ssa::tests` effect-call tests.** `set r [expr {[set x 10] + 1}]`: the effect call records a use of `x` version 1 where the test expects none. The opaque `catch`, `switch` and unseen-call tests wait on lowering shapes the walk decides.
9. **After each walk push from `analyser-binding`**, re-run and re-sort the `cfg_builder`, `ssa`, `sccp` and `optimiser` failures: most carry `<caller-frame-opaque>` and `<unseen-call>` for a command the walk did not reach, and are not this area's to fix. The walk stops recorded in `current-state.md` are: `clock seconds`, a list command over a quoted literal or a variable, `eval {…}`, `exec`, a multi-word `eval`, `incr` after `gets`, a statement after a proved error, iRules procedure bodies, method bodies.
10. **Table collapses** in `tests/it_3/optimiser.rs` and `tests/it_5/optimiser_coverage.rs`: rows that differ only in the expression can share one table test, as was done for the fused-expression and wide-shift tables in `src/sccp/`.

## Held work

Both patches apply cleanly to the restructured tree at `da039ed1` (`git apply --check` passes). Neither was compiled or tested there.

- `docs/work/name-handling/patches/handover-dataflow-optimiser-definer-may-supply-body.patch`: queue item 1, one line. Untested and uncompiled; a reading of the code against `origin/rust`, not a measured fix.
- `docs/work/name-handling/patches/handover-dataflow-optimiser-probe-harness.patch`: a local probe, never to be committed as a test. It adds `tests/it_3/zz_probe.rs` and a `mod zz_probe;` line. It compiled and ran on the pre-restructure tree; its imports may need adjusting now. Use: `PROBE_SRC='set x 1\nputs $x' PROBE_PROFILE=tcl8.6 cargo nextest run -p tcl-compiler --test it_3 -E 'test(zz_probe)' --no-capture`. Tests `zz_probe` (per function: SCCP values, types; `PROBE_CFG` dumps blocks, `PROBE_WHY` the SCCP explanations, `PROBE_VAR` a name's symbol, `PROBE_OBJ` object types, `PROBE_ESC` escape summaries, `PROBE_DIALECT_BUILD` builds through `build_for_dialect`), `zz_expr` and `zz_expr_env` (`PROBE_EXPR`, `PROBE_ENV`: the folder's answer), `zz_bytecode` (`PROBE_BC`). Before committing anything, remove the file and restore `tests/it_3/main.rs`.

No stash, local commit, side branch or worktree exists.

## Open questions and rulings

Rulings this area works under (coordinator, relayed from the owner where marked):

- **The SCCP lattice stays strict.** `sccp.values` is the execution constant that rewrites consume with no further licence. A value known only as a value is read from `FunctionUnit::semantic_values()` / the semantic facts. Tests that want "the value" read the semantic facts; do not loosen the lattice. An attempt to do so broke four tests and was reverted.
- **Proof consumers keep the strict path.** A rewrite needs a licence: `PassContext::eval_expression_at`, `expression_rewrite_equivalence_at`, or the structural licence (`structural_rewrite_equivalence`, `structural_condition_equivalence` in `src/optimiser/helpers/expr_simplify/structural_equivalence.rs`), which holds on C Tcl profiles only. Rewrites that change an operand's conversion stay advice unless the operands are proved integers.
- **F5 units fold a constant expression as Tcl 8.4 does** (owner). `eval_f5_expression` in `src/optimiser/mod.rs`. Each claim about the device is a question in `docs/work/name-handling/bigip-name-checks.md` (items 10 and 13 to 15 are this area's).
- **Bytecode target** (coordinator ruling awaiting the owner's review): structured `if` / `while` / static `eval` are lowered as on `origin/rust`; the call is kept only under a live native entry. Landed as `17abfa4f`.
- **`return` / `set` selection.** Both halves are on `work` (`a4e9d10e`, `92c87dfc`). One named regression was accepted with the second: `unit_scope::tests::agreeing_callers_still_seed_through_a_substitution`, owned by `analyser-binding`. Do not count it as new.
- **A cell linked outside the frame is a barrier** for store-to-load forwarding and propagation (`cell_is_linked_outside_frame`, `src/optimiser/propagation/load_forwarding.rs`). tclsh 8.6.18 and 9.0.4 print `traced` for `proc f {} {global x; set x 1; puts $x}` under a write trace on `::x`.
- **Dialect-blind fixtures.** A unit built from `CommandRegistry::build_default()` reads `proc` as a declaration only. Switching fixtures to `tcl8.6` mechanically fixed one test and falsely passed another; decide each fixture on its own.

Unanswered:

- Whether a qualified variable written with no `namespace eval` (`set ::ns::x 1`) in a unit with no other writer is typed or widened. `type_propagation::namespaced_scalar_keeps_qualified_symbol_type` and its array neighbour write into a namespace that does not exist, which tclsh rejects. Asked in `current-state.md`; belongs with `values`.
- `$x < $x` over an untyped operand stays advice. Not asked of anyone; it follows from the conversion rule above.

## Hand-offs

Sent and waiting:

- `analyser-binding`: the walk stops in queue item 9, and the accepted `unit_scope` regression.
- `values`: `lappend items [C new]` and a `foreach` loop variable over `List<OBJECT(::C)>` type as `OVERDEFINED` (decides `tcloo_dispatch_pattern_fixture` and `dict_for_loop_var_dispatch_resolves`). That session also said it would rename `an_unseen_call_ends_no_refinement_because_none_is_made` and add a row with a head the walk cannot resolve; check whether that landed.
- `taint`: `taint_depth::interproc_depth::sanitising_helper_result_via_var_clears_t101`, failing since `a4e9d10e`; the handler-body steps 2 to 4 recorded in `current-state.md`.
- `execution`: `tcl-codegen::it_2 codegen::catch_in_while_condition` fails on `work` with or without `17abfa4f`; that session suspected `70460540` and said it would tell the owner of that commit.

Received and not started: queue item 1 (`class-facts`) and queue item 2 (`execution`).

## Hazards

- **Disk.** The container ran out of space twice. Never use a second target directory. When space is short, delete `target/debug/incremental`; a full `cargo clean` costs a 30-minute rebuild.
- **Stale test binaries.** When copying a test binary to run it by hand, take it from `target/debug/build/tcl-compiler/<hash>/out/it_3-<hash>` found with `find … -newer`, not from an older path; a stale copy gave wrong probe results once.
- **`cargo fmt --all` reformats other areas' files** now and then. Revert anything outside this area before committing.
- **Do not edit sources while a build is in flight**; the test binary may pick up half the edit. Wait for the `Starting` line in the log.
- **A rebase changes commit ids.** Write a hash into `current-state.md` only after the push, or correct it in the next commit as happened with `92c87dfc`.
- **Generated inventory.** `make xtask-gates` failed once on `value-transfers --check` after a change in `optimiser/propagation`; regenerate and commit.
- **A fixture that looked wrong but was right:** the string comparisons in `tests/it_3/tcl_expr_eval.rs` have no release-blind answer; the release's character model decides them. And `expr {true}` is the word `true` in tclsh, not 1.
- **A fold that looks like an improvement can be wrong.** Before accepting any new fold, run the source and the optimised form on every tclsh; `prints_under_every_release` in `tests/it_2/value_transfer_witnesses.rs` does this in a test.
- **Tests that move between runs:** none seen in this area. The three slow tests finish.

## Oracle habits

- Interpreters on `PATH` (`/root/.local/bin`): `tclsh8.4` (8.4.20), `tclsh8.5`, `tclsh8.6` (8.6.18), `tclsh9.0` (9.0.4), `tclsh9.1` (9.1.0); plain `tclsh` is 9.0.4. Source trees with their test suites are under `tmp/tcl8.4.20` to `tmp/tcl9.1.0`.
- Run a one-line check on all of them: `for t in tclsh8.4 tclsh8.5 tclsh8.6 tclsh9.0 tclsh9.1; do echo "$t: $(echo 'puts [expr {1 << 70}]' | $t 2>&1)"; done`.
- `tclsh8.4` here formats wide integers unreliably (a wide shift printed -8); do not take an 8.4 wide result from it as the device's answer. Record such a case in `bigip-name-checks.md` instead.
- Tk is reachable only through Python: `xvfb-run -a python3` with `tkinter` (Tk 9.0.3), using `root.tk.eval(…)`.
- `tcllib` is not installed, and I did not use Jim in this area.
- Record the script and each release's output in the test's comment when an expectation is changed (case 3), as in `a_wide_shift_settles_what_each_release_prints` and the fused-expression table in `src/sccp/`.
