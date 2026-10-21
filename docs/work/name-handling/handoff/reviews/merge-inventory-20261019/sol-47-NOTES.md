# Sol-47: fresh-entry analysis gaps, cluster B, finding 7

Implementer: Sol-47. Branch `work` at `0e14a5722c`. Nothing committed, pushed, stashed or reset.

User decision applied: `resume-20261017-diagnosis/user-decisions-20261019T1420Z.json`. A document with no explicit execution entry is analysed as a fresh interpreter of its dialect runs it.

## Files in this directory

| File | Content | State |
|---|---|---|
| `sol-47.patch` | `git diff` of `rust/tcl-compiler`, `rust/tcl-registry`, `rust/tcl-lsp-core`, `rust/tcl-lsp-db`, `rust/tcl-lsp-server`, `rust/tcl-syntax` and `docs/design`. Only `rust/tcl-compiler` and `docs/design` have changes. | **Applied** in the working tree. |
| `sol-47-c6-companion.patch` | Sol-45's `sol-45-c6-fresh-builders.patch`, plus my test corrections for it: 2 `interprocedural` fixture helpers, one foreign-owner unit, and `optimiser_coverage` O103. | **Not applied.** It applies cleanly on top of `sol-47.patch`. It still regresses 6 tests that agree with tclsh (see §1.5). |

- Sol-46's edits in `rust/tcl-vm*`, `tcl-cmd-core` and the other paths are theirs; I did not touch them.
- My builds therefore included their in-flight `tcl-vm` state. `scratchpad/sol47/build.sh` retried while `tcl-vm` did not compile.
- Gates on the final tree:
  - `cargo fmt --check` is clean for `tcl-compiler`.
  - `cargo clippy -p tcl-compiler --lib` has no lint on a changed line. The crate still has 775 pre-existing warnings.
  - The `tcl-compiler` lib test build and the 8 integration test targets I built compile with 0 warnings.

## 1. Fresh-entry analysis gaps (task 1)

### 1a. Top-level constants: `set a 1; lindex $a 0` gave a false S100/T100/T105. **Fixed**

Root cause, found with temporary tracing:

1. **Every command looked unseen.** `CfgBuilder::head_is_unseen` selected the head's namespace with `ExecutionNamespace::for_head`. That returns `None` for a `SourceContext` frame, and the fresh entry's top level is `SourceContext(Authored("::"))`. So every literal head was treated as unseen code, including `lindex`, `puts` and the other builtins.
2. **A marker followed every call.** The CFG therefore put an `<unseen-call>` marker after each call.
3. **The marker went opaque.** `dynamic_names::scan_call` took the non-standalone original-word check on the marker. The marker has no original vector, so the check unioned `OPAQUE_SCRIPT`.
4. **The whole function went opaque.** `dynamic_names.writes` became true, so `has_dynamic_variable_trace` was set for the whole function. Every definition then became SCCP `Overdefined` while the type stayed `Int`, which is the shimmer false positive.

Fixes:

- **`cfg_builder/mod.rs`.** `head_is_unseen` now uses `for_head_context`, the identity-aware selector its sibling `variable_write_projection` already uses. The bindings queries already accept a `SourceNamespaceKey`. A receiver-selected relative head is still unseen.
- **`dynamic_names.rs`.** The scan of a non-standalone CFG now handles synthetic statements by what they are:
  - **Proved-name markers** (`CommandTokens::carries_proved_names`): iteration bindings, word effects, condition, upvar invalidation and arm writes. These run no command and state no barrier.
  - **`EvaluatedArguments`**: only the written words' substitutions matter. They are scanned under the argument-evaluation word owner (`original_argument_lexer_config`). An unproven vector still goes opaque.

Without the second fix, `foreach b $a {...}` and similar commands stayed opaque through their argument-evaluation and iteration statements.

Probe results after both fixes:

- `set a 1; lindex $a 0` gives `a = Const(Int(1))` and no S100, in both the LSP path (`for_supplied_source` + `build_with_analysis_input` + `run_all_checks`) and `build_for_profile`.
- `set a 1; foreach b $a {...}` gives `a = Const(1)`.

### 1b. O107/O112: a top-level `if {0}` was not removed. **Fixed** (via O112)

- The O112 rewrite was issued, then dropped in `finalise_optimisations` by `drop_def_elims_resurrected_by_replacements`.
- `elim_target_vars` picked the first CFG statement inside the deleted span. Under the fresh entry that is the never-entered body: a barrier with no source tokens ("original body namespace context is unavailable"). That gave `None`, which drops the edit.
- **Fix (`optimiser/manager.rs`).** For a dead-structure removal (O112), only statements the unit's solver proves executable name the definitions the removal withdraws; a complexity-guarded unit keeps every statement. A body the structure never enters defines nothing.
- O107 keeps the old behaviour on purpose. Relaxing it too exposes an O107 false positive that is currently masked, in `fp_rch_01_*`:
  - In an uncalled procedure, `while 1 { if {$c} break }` is lowered with the `if` as a generic call (see 1d).
  - The CFG then has no `break` edge, and `puts after` looks unreachable.
- The contract doc `optimiser-original-definition-resurrection-guard.md` is updated.

### 1c. O103 through an alias to `rename`: **not a soundness bug**

- The fresh analysis follows the alias. For `interp alias {} r {} rename; r p oldp; r q p; set value [p]` it folds `[p]` to **`Q`**. tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 all print `Q`.
- Variants I checked:
  - Alias to `list` folds to `P`, which tclsh agrees with.
  - A dynamic `$t` alias target folds to `Q`.
  - `rename p` with an alias prefix does not fold.
- The test asserted "no O103", which only held for the dialect-blind build. The companion patch corrects it narrowly:
  - any O103 must be `Q`;
  - the output must never contain `set value P`;
  - a new `list`-alias control must not fold to `Q`.

### 1d. Uncalled procedure bodies (`upvar 1 $n` traits and others): **not fixed**

Probe: in `proc f {} {set a 1; lindex $a 0}` with no call, each body command has:

- a lookup state;
- closed declaration advice;
- purpose `AuthoredSimulation`;
- `runtime_reachability == Conditional`.

So `proved_execution_target()` is `None`, and the following all refuse for such bodies:

- registry resolution;
- the dynamic-name scan (opaque at `invocation.is_none() && unknown residual`);
- structured lowering;
- trait collection.

When the same proc is called at top level, every command resolves.

Making Conditional bodies resolve needs a design decision. Which command world applies at a future call: the end of the document, plus whatever module-wide trust the old Standalone model assumed? I tried two narrow routes and reverted both, because they did not move these tests and the first one cost W210 (`fp_rbs_19_provably_empty_literal_still_fires`):

1. Accepting authored-simulation advice in `original_body_namespace_context`.
2. Accepting it in `original_invocation_source_purpose`.

### 1.5 C6 (`sol-47-c6-companion.patch`): measured, **not applied**

These are measured with the companion on top of this tree, unless noted:

| Filter | Base ok/total | C6 ok/total | Gains | Regressions |
|---|---|---|---|---|
| `optimiser_coverage` (integration) | 35/93 | 38/93 | 3 | 0 |
| `interprocedural::` | 55/127 | 62/127 | 12 | 4: `call_by_name_param_traits_inferred`, `forwarded_to_callee_detected`, `used_in_condition_detected`, `upvar_level_word_is_read_by_argument_parity` (all 1d) |
| `analyser::diagnostics::helpers` | 5/22 | 13/22 | 8 | 0 |
| `compilation_unit::` (partial: 94 of 98 ran) | 33/98 | — | 3 | 0 |
| `analyser::diagnostics::fp::` (earlier tree with the unrestricted O107 guard) | 442/581 | 468/581 | 31 | 5, see below |

The 5 `fp::` regressions on that earlier tree:

- `fp_rch_01_*` ×3 came from the O107 guard variant, which is now narrowed to O112.
- `fp_sh_21_dict_literal_in_dict_for_silent`:
  - The `dict for` body's entered-script observation has a frame but no namespace under the authored entry (`state.source_variables.namespace_identity` is `None`). So the body is lowered as an "unavailable" barrier.
  - Its scan then goes opaque function-wide.
  - Traced cause: the body is recorded by `record_executed_script` at the `dict for` site, argument 3, with `frame=None` and namespace `Authored("::")`. Its body entry is not `SourceBodyEntryPurpose::Selected`. `foreach` records `frame=Some`.
  - `executed_script_namespace_context` ignores frameless observations, so lowering finds no namespace.
  - Deriving the namespace from the frame layout did not help, because there is no frame; reverted.
  - The fix belongs where `dict for` selects its body entry. I did not change that.
- `fp_tnt_03_interp_eval_list_literal_head_no_t105`:
  - `$child` is never set, so in a fresh interpreter `interp eval $child [list puts $raw]` errors before `[list …]` runs.
  - The precise fresh analysis cannot prove the nested `list` invocation, so the suppression does not apply.
  - The sink is equally unreachable.

Remaining blockers before C6 can be applied: 1d (4 tests), the `dict for` body namespace (1), and `fp_tnt_03` (1).

The 7 authored-body contract tests in `interprocedural` are covered by the companion's corrections:

- They now build dialect-blind, which is explicit Standalone authoring, as Sol-45 did for 3 others.
- One more: `authored_source_report_labels_refuse_foreign_and_changed_owners`, whose foreign-owner unit also built through `build_for_profile`.

## 2. Cluster B: whole-module compile without an entry (task 2). **Fixed for the compile service**

- **Fix (`compile_service.rs::compile_target_with`).** A `WholeModule` compile with no native entry now takes the fresh supplied-source entry over the profile's complete input:
  - `fresh_document_input`: the profile's default availability over the caller's exact store, checked with `InvocationMetadataContext::for_source_input`;
  - then `SourceAnalysisEntry::for_supplied_source(..).options()`, with the whole-module scope.
- This is the same entry `diagnostic_report` uses.
- A supplied runtime entry keeps its own options. With no profile, or with metadata that does not correspond, the old path and its refusal stay.
- An earlier variant, which simply used the entered-source authoring options, regressed `compile_service::tests::entered_source_scope_excludes_deferred_bodies_before_cfg_preparation`: whole-module procedures were no longer pre-lowered. The final variant does not.

| Measure | Before | After |
|---|---|---|
| `tcl-compiler` lib `compile_service::` | 16/48 | 20/48 (+4, 0 regressions) |
| `tcl-debugger` lib (the hang test skipped) | 7/17 per Sol-44 | **17/17** |
| `tcl-vm` `mathfunc_derivation_e2e` | 2/9 per Sol-44 | **7/9** |
| `tcl-vm` `word_value_resolution_e2e` | 1/3 | 1/3 |
| `tcl-fuzz` bins | 93/106 | 93/106 |

Notes on the rows:

- **`mathfunc_derivation_e2e`.** The 2 remaining failures are semantic:
  - 8.6 canonicalisation of `0x10` is not applied (`[V8_6] ["16","16","16","0x10","0x10"]`);
  - 8.4 `nextafter` gives "syntax error in expression" where "unknown math function" is expected.
- **`word_value_resolution_e2e`.** The failures are no longer refusals. `string length "{}$z"` gives 4; tclsh gives 3. This compiled-path miscompile was hidden behind B.
- **`tcl-fuzz` bins.** Unchanged, because the `wasm_diff` direct arm and `generator` use `lower_to_ir` + `codegen_module`, not the compile service.

Harnesses that still use `lower_to_ir` + `codegen_module` still refuse. These include `run_script`, `builtins_e2e` and the `tcl-fuzz` direct arm. Moving them to `BytecodeCompileService::for_profile(..).compile` would pick up this fix. I did not change `lower_to_ir`: it is the shared test-fixture entry for the whole workspace.

## 3. Finding 7 and the `per_item` stamping cost (task 3). **Partial: still quadratic**

Changes:

- **`word_expr.rs`.** `original_run_end` reuses the caller's `SourceMap`. Before, it built a whole-buffer `SourceMap::new` (a line index over the full text) for every word. `word_span` reads only the bytes.
- **`retained_body_inventory.rs`.** The completed, read-only lowering inventory keeps a bounded memo (4096 entries) of exact-offset `invocation_at_source` projections, used by `stamp_original_tokens`.
  - This is the same contract as the realm memo: `original-invocation-projection-memo.md` is updated.
  - Before, each enclosing level re-derived every descendant point.
- **`codegen/values.rs` and `codegen/mod.rs`.** `intern_script_locals` keeps a per-compile memo. It is keyed by script text and normalised lexer config, and shared with the nested compiles it makes. A body nested `n` deep is now compiled for its locals once, not once per enclosing level.

Measured with `BytecodeCompileService::for_profile(tcl9.0).compile`, the debugger's path, on `n` nested `if {1} {`:

| Depth | Before | After | Lowering phase | Codegen phase |
|---|---|---|---|---|
| 16 | 6.5 s | 5.6 s | 0.26 s | 5.2 s |
| 32 | 27.9 s | 22.2 s | 0.42 s | 21.6 s |

- Growth is still about quadratic, and codegen dominates.
- With all three memos in place, a gdb sample shows the time inside each nested intern compile's lowering. `lower_proc_body_module_for_bytecode` runs a full `SourceCommandBindings` analysis of each body. Its own stamping goes through `SourceCommandBindings::stamp_original_tokens` directly, and the builder deliberately keeps no memo there.
- `stamp_original_tokens_with_bindings` also attaches every descendant dispatch point to each enclosing token vector, so the total is O(n²) bindings.
- The 400-deep `tcl-debugger` test was not run. Extrapolating gives about an hour.
- Next steps:
  - stop attaching braced-body descendants to the enclosing vector (needs a consumer audit);
  - or derive a body's compiled locals without a full binding analysis.

`per_item`:

- `many_small_bodies_totalling_over_the_threshold_stay_fast` fails in 432 s. It asserts the fast path, and it gets `Some(OriginalSourceOwnership)`: Luna-37 C7(i), the fallback predicate. The base run finished in 358 s; its status was not captured.
- `ordinary_large_body_stays_on_the_fast_path` timed out at 900 s at base. The after run was still running at 1259 s when I stopped it while switching binaries, so there is no result.

## 4. Cluster C String compilers (task 4): not started

Disassembly on tclsh 8.5/8.6/9.0/9.1 is captured in `scratchpad/sol47/dis-*.txt`. What it shows on 8.6:

| Command | Bytecode |
|---|---|
| `string range` | `strrangeImm` for literal indices, `strrange` otherwise |
| `string map` with a literal 2-element map | `strmap` |
| `string replace` | `strreplace`; 3 operands compile to `dup`/`strrangeImm`/`strcat` |
| `string is integer` | a `numericType` sequence |
| `string map` other forms | `invokeStk ::tcl::string::map` / `invokeReplace` |

## 5. Compiler half of fcb7927f02 (task 5): already on `work`

All of the following are present:

- `terminator.rs::name_condition_command` (`enclosing`, `ErrorOrigin::Execution` for direct statements);
- `codegen/mod.rs::in_direct_statement`;
- the `expressions.rs` no-fold rule;
- `TclVersion::keeps_expression_substitution_errors_logged`.

`tcl-vm-cli` is unchanged on the final tree: `script_errors` 3/5 and `top_level_e2e` 6/10. The vectors that still fail are therefore not the compiler half.

## Measurements

### Method

- **Baseline for lib filters:** a HEAD `tcl-compiler` lib test binary built at the start, with Sol-46's `tcl-vm` state at that time.
- **Final:** this tree.
- **Integration baseline:** Sol-45's `scratchpad/sol45/res/base-*.txt`. Sol-45 measured those binaries unchanged by its committed Task 1, and none of them uses `BytecodeCompileService`.
- **Skipped in lib runs:** `memoized_`, `tcltest_aliases_preserve`, `depth_far_past_cap`, `ordinary_large_body`, `many_small_bodies` and `literal_flows_to_regexp_in_a_large`.
- **Environment:** `TCL_LSP_REQUIRE_TCLSH=9.0,8.6` and `TCL_LSP_JIMSH`.

### `tcl-compiler` lib module filters (base vs final)

| Filter | Base ok/total | Final ok/total | Gains | Regressions |
|---|---|---|---|---|
| `analyser::bounds_checks` | 55/68 | 55/68 | +0 | 0 |
| `analyser::diagnostics::fp` | 442/581 | 445/581 | +3 | 0 |
| `analyser::diagnostics::helpers` | 5/22 | 5/22 | +0 | 0 |
| `analyser::diagnostics::security` | 28/36 | 28/36 | +0 | 0 |
| `analyser::diagnostics::tests` | 579/830 | 580/830 | +1 | 0 |
| `analyser::per_item` | 54/68 | 54/68 | +0 | 0 |
| `cfg_builder` | 86/262 | 86/262 | +0 | 0 |
| `codegen` | 515/552 | 515/552 | +0 | 0 |
| `command_binding` | 860/1176 | 860/1176 | +0 | 0 |
| `compilation_unit` | 33/98 | 33/98 | +0 | 0 |
| `compile_service` | 16/48 | 20/48 | +4 | 0 |
| `dynamic_names` | 29/48 | 29/48 | +0 | 0 |
| `gvn` | 58/94 | 58/94 | +0 | 0 |
| `interprocedural` | 55/127 | 55/127 | +0 | 0 |
| `irules_checks` | 55/116 | 55/116 | +0 | 0 |
| `lowering` | 229/405 | 229/405 | +0 | 0 |
| `lowering_hooks` | 22/28 | 22/28 | +0 | 0 |
| `optimiser` | 339/558 | 339/558 | +0 | 0 |
| `realm` | 34/43 | 34/43 | +0 | 0 |
| `registry_invocation` | 207/287 | 207/287 | +0 | 0 |
| `sccp` | 123/202 | 123/202 | +0 | 0 |
| `shimmer` | 93/214 | 94/214 | +1 | 0 |
| `ssa` | 156/229 | 157/229 | +1 | 0 |
| `taint` | 43/62 | 43/60 | +0 | 0 |
| `word_expr` | 8/8 | 8/8 | +0 | 0 |

Tests that passed at baseline and fail on the final tree: **0** in every filter above. `taint::` aborts at the same test at base and final (62 and 60 results listed), as Sol-45 also saw.

### Integration binaries (ci-5201 lists)

| Binary | Base ok/total | Final ok/total | Regressions |
|---|---|---|---|
| optimiser_coverage | 35/93 | 36/93 | 0 (+1) |
| checks | 236/274 | 236/274 | 0 |
| dataflow | 3/45 | 3/45 | 0 |
| codegen_depth | 20/55 | 20/55 | 0 |
| var_escape_typeinfer | 19/70 | 19/70 | 0 |
| taint_depth | 50/130 | 50/130 | 0 |
| analyser | 339/506 | 339/506 | 0 |
| taint | 159/373 | 159/373 | 0 |

`tcl-lsp-core --lib diagnostic`: base 109/137, final 109/137, 0 regressions.

- The baseline was built by reverse-applying `sol-47.patch`, building, and re-applying it. The diff was identical afterwards.
- Nothing in `tcl-lsp-core`'s diagnostic tests covers the top-level S100 case that 1a fixes on the LSP path. The fix was verified with an in-crate probe of the same path (`for_supplied_source` + `build_with_analysis_input` + `run_all_checks`).

## Disk and housekeeping

- Disk fell below 2 GB twice, with other agents building as well. Each time I deleted superseded workspace-crate rlib/rmeta files in `target/debug/deps`: anything beyond the 3 newest per crate and older than 2–3 h, about 1.0 GB and 2.9 GB. Cargo rebuilds them on demand.
- I deleted every test executable after its run.
- Scratch, all under `scratchpad/sol47/`:
  - `res/` holds the per-test lists;
  - `build.sh`, `runlib.sh`, `batch.sh`, `runint.sh` and `cmp.sh` are the runners;
  - `sol47_probe.rs.bak` is the removed probe module;
  - Every test binary has been deleted.
