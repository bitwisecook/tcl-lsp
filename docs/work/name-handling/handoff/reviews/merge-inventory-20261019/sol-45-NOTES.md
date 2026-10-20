# Sol-45: cluster C (native String compilers), C6 (standalone builders), C2 helpers

Implementer: Sol-45. Branch `work` at `bf3bc3b5e2`, uncommitted. Nothing committed, stashed or reset.

## Patches in this directory

| File | Content | State |
|---|---|---|
| `sol-45.patch` | `git diff -- rust/tcl-compiler/src rust/tcl-registry docs/design` plus the new untracked file `rust/tcl-registry/src/native_string_operand_compilation.rs` | Applied in the working tree: cluster C String compilers and the C2 test ingress. |
| `sol-45-c6-fresh-builders.patch` | The C6 builder change, its three contract-test corrections and the contract paragraph | **Not applied.** Implemented, measured, then reverse-applied because it regresses tests that agree with tclsh (see §2). |

- Sol-44's files are not mine and I left them alone: `rust/tcl-compiler/tests/wasm_codegen.rs`, `rust/tcl-vm*`, `tcl-cmd-core`, `tcl-engine-tclvm` and others.
- `cargo check -p tcl-registry -p tcl-compiler --lib --tests`: 0 warnings.
- `cargo fmt --check` is clean for `tcl-registry` and `tcl-compiler`.
- Clippy was not run, because CPU and disk were shared.

## 1. Cluster C: root cause, and fix for the fixed-operand String members

### Root cause

This is **not a merge regression**. The merge's `string_.rs` hook additions are unrelated.

Work's contract (`docs/design/contracts/resolved-semantic-queries.md`) says: "a missing descriptor never licenses generic dispatch; `HookFrom`/`Unresolved` keep an actual C hook's grammar unknown". Selection fails like this:

1. On a native-core VM, `string` is an ensemble. Selection goes `actual_ensemble_plan` → `compile_original_selected_worker`.
2. The VM's hook presence for `::tcl::string::<member>` comes from `CommandRegistry::native_compilation_for_registration`.
3. Every member except `length`, `equal`, `match`, `repeat` and `trim*` had no descriptor, so:
   - hook presence is `Unknown`;
   - the worker is `Unavailable`;
   - the selection is `Unknown`;
   - `codegen/mod.rs:877` refuses;
   - the result is `CompilationAdmission(NativePreflightRequired)`.
4. `format`, `regsub`, `encoding` and the other `HookFrom(v)` commands fail the same way. A real C hook exists for each, but its grammar is not authored.

Work never admitted these. Before the harness migration, clusters A and B hid it.

### Fix

New pieces:

- grammar `NativeCompilationGrammar::StringOperand { scope, operation }`;
- module `tcl-registry/src/native_string_operand_compilation.rs`;
- plan `NativeInstructionPlan::StringOperand`;
- codegen `append_native_string_operand_tasks`, which emits the selected stack op with the native switch version;
- the preflight operand preparation for the new plan.

The member grammars follow the C source (`tclCompCmdsSZ.c`, `tclEnsemble.c`; for 8.4, `TclCompileStringCmd`). They were measured with `::tcl::unsupported::disassemble script` on tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0:

| Member | Inline from | Inline op | Fallback for other operand counts |
|---|---|---|---|
| `compare`, `index` | 8.4 | `strcmp` / `strindex` | 8.6+: `invokeReplace` (ensemble rewrite); 8.4 and 8.5: generic |
| `first`, `last` | 8.6 | `strfind` / `strrfind` | `invokeReplace`. 8.5 has a NULL compileProc, so the command runs generically. |
| `toupper`, `tolower`, `totitle` | 8.6 | `strcaseUpper` / `strcaseLower` / `strcaseTitle` | 2–3 args: `TclCompileBasic1To3ArgCmd`, a direct `invokeStk` of `::tcl::string::<m>`. 0 or 4+ args: `invokeReplace`. |

Also:

- `{*}` makes every member generic (`invokeExpanded`).
- `reverse`, `wordend` and `wordstart` become `NamedEnsembleInvocation` from 8.6, like `repeat`. These are the Basic1Arg/Basic2Arg compilers: a direct invoke of the full private name.
- New registry unit tests (3) pin these windows. All pass.

### Results

| Measure | Before | After |
|---|---|---|
| `tcl-vm` `cmd_string_e2e` (native core) | 6/51 | **13/51** |
| `tcl-registry` lib | 1872 ok / 20 fail | same; all 20 failures are in the ci-5201 lists, none String-related |
| `tcl-compiler` lib modules | | 0 regressions (table below) |

The 7 newly passing `cmd_string_e2e` tests are case conversion, case-conversion errors, compare/equal errors and options, first, last, and subcommand dispatch.

### Remaining in cluster C

Each item needs an authored C grammar. Each still refuses today.

- `string range`: `strrangeImm` with literal-index encoding, plus the `pop; push ""` cases for end+1 and -1; otherwise `strrange`. The lrange encoder `tcl_cmd_core::index::compiled_list_bound_in` can be reused.
- `string map`: `strmap` for a literal 2-element map.
- `string replace`, `string is`, `string cat`, and `string insert` (9.0).
- `format`: `TclCompileFormatCmd`.
- `regsub` (including 9.0 `-command`), `encoding`, and the other `HookFrom` commands.
- The "proc with an execution trace" refusal: not reproduced. `command_traces_e2e`'s 2 failures are VM-side (`ValueAccessRefusal`, owner snapshot), not cluster C.

`cmd_string_e2e` still has 38 failures from these: range ×1, map, replace, is ×7, insert, and format ×21+.

## 2. C6: implemented, measured, and **not applied**

### What the change does

The change is in `sol-45-c6-fresh-builders.patch`:

- A dialect-bearing default build goes through `CompilationUnit::build_with`. That covers `build_for_dialect`, `build_for_profile`, `build_with_options` and the memoised variants.
- Such a build now creates the dialect's complete input: default availability over the caller's exact store, plus the caller's config.
- It validates that input with `InvocationMetadataContext::for_source_input`.
- It then takes the supplied-source fresh-interpreter entry (`SourceAnalysisEntry::for_supplied_source`), as the user decision of 2026-10-19T14:20Z requires. So `proc f` publishes `::f`.
- Dialect-blind builds and non-corresponding stores keep Standalone authoring.

### Measured gains

`helpers` +8 (the `unit.function("::f")` panics), `fp` +26, `compilation_unit` +3, `gvn` +2, `lowering_hooks` +1, `codegen` +7, `lowering` +14, `sccp` +7, `ssa` +14, `optimiser` +15, `registry_invocation` +4, `interprocedural` +12.

### Measured regressions (tclsh agrees with every one of these tests)

These are fresh-entry (C1) analysis gaps. The LSP `diagnostic_report` / `run_all_checks` path has used this entry since Sol-37/42, so I expect it to show the same gaps.

- **Top-level constants**:
  - Behaviour: under the fresh entry, a top-level `set a 1` gets type `Int` but SCCP `Overdefined`. Procedure bodies fold.
  - Effect: `shimmer::hints::is_pure_value` sees a committed int, which gives a false S100/T100.
  - Failing tests: `fp_sh_21_scalar_literal_as_list_silent`, `fp_sh_21_dict_literal_in_dict_for_silent`, `fp_sh_23_committed_element_arithmetic_silent`. `fp_tnt_03_…_no_t105` (false T105) is likely the same cause.
  - Repro: `set a 1; lindex $a 0` under tcl8.6.
  - Ruled out: var-observability escaping, module traces and deferred writes (all empty); `source_variable_symbol`; command mutations (`set` is the builtin).
  - Next suspect: the typed-assignment path in `sccp::evaluate_def_dispatch` / the top-level original store carrier.
- **Optimiser** (`optimiser_coverage`):
  - `o107_dead_code_in_false_branch` and `o112_nested_via_multipass`: a top-level `if {0}` is no longer removed. `O100` is still found, but the O112 rewrite is not issued.
  - `o103_alias_resolved_proc_replacement_is_not_folded_as_the_original`: O103 is now emitted for `interp alias {} r {} rename; r p oldp; r q p; set value [p]`. tclsh returns `Q`. **This is a soundness bug**: an alias to `rename` is not treated as a command-table mutation.
- **Interprocedural**: 11 regressions. Some are functional:
  - `call_by_name_param_traits_inferred`: the `upvar 1 $n x` VarWrite/VarRead traits are lost;
  - `forwarded_to_callee_detected`, `used_in_condition_detected`, `upvar_level_word_is_read_by_argument_parity`.
  - The rest pin the authored-body contract (`authored_source_report_labels_*`, `original_authored_*`, `original_formal_reference_traits_*`, `conditional_authored_declaration_*`). Those can be narrowly corrected the same way as the 3 contract tests the patch already corrects, by building dialect-blind.

### Decision needed

The C6 change is only safe once the fresh-entry analysis closes the gaps above:

- top-level SCCP folding;
- O107/O112 issuance;
- alias-to-rename mutation;
- upvar parameter traits.

Those gaps already affect the LSP. Fixing them first fixes both. The coordinator can apply the patch at that point.

## 3. C2: helpers moved onto the checked `analyse` ingress; the product gap remains

- `Analyser::emit_cfg_ssa_diagnostics_standalone_for_test` (the "explicit legacy fixture ingress") is replaced by `emit_document_cfg_ssa_diagnostics_for_test`.
  - It runs `Analyser::analyse(src, <the analyser's profile>)`: the checked complete-input path with the supplied-source entry.
  - It keeps only the diagnostics added by the CFG/SSA phase. These are captured through the test-only `Analyser::cfg_ssa_capture_for_test` around `emit_cfg_ssa_diagnostics` in `run_diagnostic_emitters`.
  - A supplied `cu_override` is still consumed directly.
- All 43 call sites move with it, including `codes_for` and `w210_fires_for`. No assertion changed.
- `analyser::diagnostics::tests` goes from 578 to 579 ok (`emit_cfg_ssa_diagnostics_w211_unused_variable`), with 0 regressions.
- The C2 members still fail under the default plain `tcl` (Logical) profile, because the dynamic-name barrier still goes opaque. For example, `codes_for("proc f {} { puts $u }")` has no W210, and `info_exists_folds_false_for_never_defined_local` gives `["W214"]`.
- **Product part not done.** Expression-embedded `[..]` and deferred-body commands need original written-word sites, so that `original_lexer_config_for_tokens` succeeds. The existing attachment point is `OriginalBodyWrittenWordOwner::stamp_original_tokens`.

## 4. C3: not started

## Measurements

### Method

- **Baseline:** HEAD `bf3bc3b5e2` plus Sol-44's tree at 17:15Z, with my changes reverse-applied, built once.
- **After:** the final applied tree (Task 1 + C2 helper).
- **Skipped tests:** `memoized_`, `tcltest_aliases_preserve`, `depth_far_past_cap`, `ordinary_large_body`, `many_small_bodies`.
- **Environment:** `TCL_LSP_REQUIRE_TCLSH=9.0,8.6` and `TCL_LSP_JIMSH`.

### `tcl-compiler` lib module filters, final applied tree

| Filter | Base ok/fail | After ok/fail | Regressions |
|---|---|---|---|
| `analyser::diagnostics::tests` | 578/252 | 579/251 | 0 |
| `analyser::diagnostics::helpers` | 5/17 | 5/17 | 0 |
| `codegen::` | 515/37 | 515/37 | 0 |
| `registry_invocation` | 207/80 | 207/80 | 0 |
| `interprocedural::` | 55/72 | 55/72 | 0 |
| `command_binding::` | 860/316 | 860/316 | 0 |
| `lowering::` | 229/176 | 229/176 | 0 |
| `compilation_unit::` | 33/65 | 33/65 | 0 |
| `sccp::` | 123/79 | 123/79 | 0 |
| `ssa::` | 156/73 | 156/73 | 0 |
| `gvn::` | 58/36 | 58/36 | 0 |
| `lowering_hooks::` | 22/6 | 22/6 | 0 |
| `analyser::diagnostics::fp::` | 442/139 | 442/139 | 0 |
| `optimiser::` | 339/219 | 339/219 | 0 |

`analyser::diagnostics::security` (28/8), `irules_checks::` (55/61) and `taint::` (a partial run that aborted at the same test at base and after, 43/17) also show no change. All three were measured with the Task 1 + C6 tree.

### Integration binaries (ci-5201 list), ok/total

The integration "after" runs were built from the Task 1 + C6 tree (the 12 rows below). `optimiser_coverage` was re-run on the final tree.

| Binary | Base | After (1+C6) | Regressions |
|---|---|---|---|
| taint | 159/373 | 159/373 | 0 |
| checks | 236/274 | 236/274 | 0 |
| optimiser_coverage | 35/93 | 32/93 | 3, all from C6 (see §2) |
| var_escape_typeinfer | 19/70 | 19/70 | 0 |
| analyser | 339/506 | 339/506 | 0 |
| taint_depth | 50/130 | 50/130 | 0 |
| dataflow | 3/45 | 3/45 | 0 |
| codegen_depth | 20/55 | 20/55 | 0 |
| optimiser_coverage, **final tree** | 35/93 | 35/93 | 0 |

## Shared-tree and disk notes

- An earlier batch wrote raw Debug output from failing tests into `scratchpad/sol45/res` and filled the disk; the coordinator truncated it.
- The runner (`scratchpad/sol45/runlib.sh`) now keeps only the test, panic and assert lines, capped at 20 MB.
- `scratchpad/base/tcl_compiler_lib` (247 MB) is the baseline binary. Delete it when no longer needed.
