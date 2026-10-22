# compiler-codegen: entry-less whole-module compiles, pack compiler rows, `string cat`, dict release units, N17

Implementer: compiler-codegen agent. Branch `work`. I committed, pushed, stashed and reset nothing; the
coordinator's WIP snapshots carry the tree as it stood (e.g. 56893780b9, bc79dd0d4d).

User decision applied: `user-decisions-20261019T1420Z.json` — a document or module with no explicit
entry runs as a FRESH INTERPRETER OF ITS PROFILE (`tclsh file.tcl`).

## Files

| File | Owner | Change |
|---|---|---|
| `rust/tcl-compiler/src/compile_service.rs` | mine | fresh whole-module path (§1), profile-less engine, manifest restamp, Direct fallback (§2); 6 regression tests |
| `rust/tcl-compiler/src/codegen/control_flow.rs` | mine | `release_compiled` for `dict update` / `dict with` (§4) + test |
| `rust/tcl-compiler/src/codegen/emitter/mod.rs` | mine | `module_manifest` is `pub(crate)` |
| `rust/tcl-compiler/src/codegen/statements/native_scalar.rs` | mine | `string cat` emission (§5) |
| `rust/tcl-compiler/src/codegen/native_failure.rs` | mine | two contract tests restated for the user decision (§1.4) |
| `rust/tcl-compiler/src/command_binding/compiled_preflight.rs` | granted by coordinator | entry-less provider rule (§1); `command` split into plan + descent frames (§7) |
| `rust/tcl-compiler/src/command_binding/compiled_preflight/original_preparation.rs` | granted | fresh-world expression emission (§1) |
| `rust/tcl-compiler/src/const_subst.rs` | granted by decision 2026-10-22T13:17Z | declared-world-footprint-only barrier; implementation pending unchanged-probe measurement |
| `rust/tcl-compiler/src/optimiser/manager.rs` | was git-clean | `optimise_raw(.., None)` over a profile-projected registry analyses for that profile (§6) |
| `rust/tcl-spectcl/src/install.rs` | granted | derived native compilers for pack rows (§3) + 2 tests |
| `rust/tcl-spectcl/tests/variable_reference_boundaries.rs` | new | N17 regressions (§8) |
| `rust/tcl-registry/src/native_string_operand_compilation.rs`, `native_compilation.rs`, `commands/tcl/string_.rs` | granted (string cat) | `string cat` grammar (§5) + test |
| `runtime/rust/src/interp/native_body_artifact/native_string_operand.rs` | granted (string cat) | `Cat` arm |

The compiler agent's fix already in my files (`codegen/cmd_subst.rs::emit_native_argument_word`, `string length
"{}$z"`, test `compile_service::tests::native_operand_words_in_a_substitution_keep_their_variables_live`) is intact;
it passes before and after. tcl-vm `word_value_resolution_e2e` stays 3/3.

## 1. Root cause of contracts (1)–(3), and the fix

Probe matrix (tcl9.0, `set x 1; lassign {1 2} a b; proc caller {n} {..}; list $a $b`), current tree:

- e34d8a427e's fresh entry took `SourceAnalysisEntry::for_supplied_source(..).options()`, whose `native_compilation`
  is `authoring_native_compilation()` = mode **Direct** ("evaluate each command directly, never compile"). Every
  command then selects `Generic` → plain `INVOKE_STK1`, `command_bindings=[]`, no inlining. `tclsh file.tcl` compiles
  (`Tcl_EvalObjEx` without `TCL_EVAL_DIRECT`), so the right mode is **BytecodeObject**.
- Only `set_source_analysis_options` was applied, not `Lowerer::with_resolved_analysis_input`, so the lowering had no
  `metadata_input`/`dialect_context` → `lower_proc` never published procedures (`procs=[]`).
- With BytecodeObject + resolved input: `set`→AssignConst, `lassign`→Inline(ListAssign), `list`→Inline,
  `procs=[::caller]` — and the only remaining blocker was the chunk's `provider_required`, from
  `compiled_preflight::preflight_image`: `native_entry_unavailable = state.baseline.native_entry.is_none()` — every
  entry-less BytecodeObject chunk required a provider. The same flag made `inlining::script_requires_admission`
  refuse reference-body inlining, and made the raw pipeline (contract 3) and `default()` (contract 1) refuse.

Fix:

1. `compiled_preflight.rs`: `native_entry_unavailable = native_entry.is_none() && baseline.unknown_entry`. An
   entry-less compile is the fresh interpreter of its profile; an explicit unknown entry keeps the obligation;
   `possible_error` (unsupported compiler visits, math-table mismatch) still sets `provider_required`; live-entry
   compiles are unchanged.
2. `original_preparation.rs`: with no live entry and no unknown history, an expression program's emission is judged
   against the fresh profile's own policy (`native_expression_evaluation_policy(profile, point)` +
   `expression_program_emission_for_policy`) instead of always `Unavailable`. This was what kept `expr {1 + 2}`
   `ProviderRequired` at every release.
3. `compile_service.rs::compile_target_with`: a whole-module, entry-less compile lowers through
   `lower_fresh_document` = `Lowerer::with_resolved_analysis_input` + the fresh entry's options with
   `NativeCompilationMode::BytecodeObject` (hosted/F5 inputs keep the fresh rule loader's own Direct protocol).
4. `codegen/native_failure.rs`: `missing_original_entry_keeps_compiler_rejection_as_an_admission_obligation` and
   `missing_original_entry_does_not_invent_a_guest_compiler_error` pinned the old "no entry ⇒ provider required"
   contract that the user decision replaces. They now assert it for an **explicit unknown entry**
   (`unknown_entry_*`, unchanged assertions), and a new test asserts the entry-less fresh 8.4 compile presents the
   compiler's own rejection (`wrong # args: should be "set varName ?newValue?"`), equal to a captured fresh 8.4
   entry's, and admits.

## 2. Profile-less services, manifest, Direct fallback

- `BytecodeCompileService::default()` (profile `None`) compiles for the engine its modules run under:
  `plain_tcl`'s `vm_runtime_version` (9.0) via `resolve_environment(..).unit_profile()`, over that engine's
  projection of its owned registry. `plain_tcl` itself has no core point, so every compiler selection was `Unknown`.
  The module is still stamped `plain_tcl` (the profile the VM pins profile-less modules to), and its manifest is
  recomputed for that profile (it said tcl9.0: "bytecode manifest disagrees … compiled for tcl9.0, runtime holds
  tcl" in 3 codegen_stamps tests).
- Direct fallback: when the fresh BytecodeObject compile is refused at admission (a command the catalogue has no
  compiler descriptor for, e.g. the `::tcl::mathfunc::*` rows), the document is compiled again evaluating each
  command directly — e34d8a427e's behaviour. Found because `mathfunc_derivation_e2e::integer_conversions_preserve_the_operand_object`
  went pass→fail without it; with it, 7/9 → 7/9.

## 3. Pack rows (tcl-spectcl `install.rs`)

Pack-installed rows had `native_compilation: None` (the loader refuses pack-authored native contracts), so under a
BytecodeObject compile every pack command selected `Unknown` → provider required / codegen refusal (`llength`
-override, `vendor::unpack`, `vdouble`). Derived facts for a fresh interpreter with the pack loaded:

- an `-override` of a shipped builtin keeps the shipped row's compiler (command and each subcommand);
- a command the pack adds (a `proc` or `Tcl_CreateObjCommand`) has no compileProc → `NoHook` / `Invoke` / `Inherit`,
  spelled as shipped `NoHook` rows are;
- an `alias_of X` with an admitted bundled codegen stamp takes `X`'s compiler from the registry it is installed
  into; without the stamp it is an alias command → `NoHook`.

The loader still refuses pack-authored `native_compilation`. Tests: `pack_rows_take_the_compilers_a_fresh_interpreter_holds`,
`an_admitted_alias_stamp_takes_its_targets_compiler`.

## 4. `release_compiled` for `dict update` / `dict with` (item 4)

`codegen/control_flow.rs::release_compiles_dict_scope`, measured with `::tcl::unsupported::disassemble`:
8.5.19 compiles `dict update d k v {}` in a proc (`dictUpdateStart`), not at top level, not with `::d` or `a(1)`;
8.6.18 / 9.0.4 also compile `dict with d {...}` and `dict with ::d {...}` in a proc (`dictExpand`), and only an empty
`dict with` body at top level. Locals the compiler makes are interned. `switch`/`try`/`dict for|map` already set it.
Test `the_release_compiles_dict_update_and_dict_with_in_a_procedure_body`.

## 5. `string cat` native compiler

`TclCompileStringCatCmd` (8.6+, verified on 8.6.18 / 9.0.4 / 9.1.0; 8.5 has no `string cat`): any operand count;
none pushes `""`, one is the word, otherwise `strcat` with each run of compile-known words folded into one literal,
flushing at 254 operands. New `NativeStringOperandOperation::Cat` (`accepts(count)` replaces `arity()`),
`literals` on `NativeStringOperandInstruction`, descriptor on the `cat` subcommand, codegen emission, runtime/rust
`Cat` arm (`tcl_cmd_core::string::cat`). Tests: registry `string_cat_compiles_from_8_6_at_every_operand_count`,
compile_service `string_cat_compiles_with_its_literal_runs_folded`.

`string is`: NOT done. C's `TclCompileStringIsCmd` has three release variants (8.6 vs 9.0: `dict` class added;
9.1: `IS_EMPTY`, `SWAP`, different `is false` sequence), per-class jump sequences and catch ranges for
`list`/`dict`. It needs its own grammar, plan, preflight arm, codegen emitter and runtime arm. `string map`,
`range`, `replace`, `insert`, `format` likewise remain (Sol-45 list).

## 6. const_fold / O129

Probe: `optimise_raw(src, registry, None)` folded nothing, not even `[string length abcde]` at top level; with
`Some("tcl8.6")` both the top-level and the uncalled-proc forms fold to `5`. The pack tests pass `None` over a
registry projected for tcl8.6. `optimise_raw` now resolves `None` to the registry's own profile
(`registry.profile()`); a profile-less registry stays dialect-blind. Pack rows also needed compiler descriptors (§3)
before a pack command's proved target closes. Live-hook regression in §8.

## 7. CompilerTraversal frame size

`command` is split: `command_visit` (resolution: target, facts, shapes, selection, recipe, steps — returns a boxed
`CommandVisit`) runs and returns before the descent; `command` and `compiled_command` dispatch with references into
the box; `script` builds its `CommandTokens` on the heap. Debug-build frames (objdump): before `command` 13,592 B,
`script` 4,712 B; after `command` 1,576 B, `command_visit` 7,896 B (not on the recursion path), `compiled_command`
measured 6,136 B before its last shrink (preparation now borrowed, not moved) — re-measure pending.

## 8. N17 (review naming-migration-review-20261022)

The current tree already decomposes an array key with the real lexer (`push_array_key` → `ExecutablePartArena`;
`emit_variable_reference` → `is_whole_variable_reference`); the triage agent confirmed the template case is `good`
in all 16 combinations. Regressions added:

- `compile_service::tests::a_variable_reference_ends_where_the_release_scanner_ends_it`: compiled and run in the
  VM at 8.4–9.1, top level and in a proc: `$arr($a:b) $arr(${a}:b) $arr($a:) $arr($a::b) $arr($a:::b) "$a:b" "$a:"
  "$a.b"` → `good|good|lone-good|ns-good|ns-good|x:b|x:|x.b|x:b` (tclsh 8.4.20–9.1.0 agree; Jim reads `$a:::b` as
  the name `a:::b`, so that case is Tcl-only).
- `rust/tcl-spectcl/tests/variable_reference_boundaries.rs`: template × DSL 1/2.0 × direct/helper proc × fast path
  on/off for `$arr($a:b)`, `$arr(${a}:b)`, `$arr($a:)` → `good`; colon runs `$arr($a::b)`, `$arr($a:::b)` → `good`;
  live const_fold hook (production `pack::load` → `hooks::plan_for` → `tclvm_host` → `registry_for_dialect_with_packs`
  → `optimise_raw`) × DSL 1/2.0 × `$arr($a:b)` / braced control → O129 `good`.

## Measurements

Method: one binary per target; "base" runs with the TEMP env toggle `ZZ_CCG_BASELINE=1`, which restored every
behaviour I changed (e34d8a427e fresh path, old provider rule, no fresh expression emission, no dict release units,
`string cat` Unknown, no pack-row compilers, dialect-blind `optimise_raw`); "after" without it. The toggle and the two
`zz_ccg_*` probe tests are deliberately retained until the unchanged live-fold probe is measured; removal is pending. `TCL_LSP_REQUIRE_TCLSH=9.0,8.6`, `TCL_LSP_JIMSH`.

| Target | Base ok/total | After ok/total | pass→fail |
|---|---|---|---|
| tcl-compiler lib `compile_service::` (first run, pre-toggle baseline binary) | 21/49 | 35/53 | 0 |
| tcl-compiler lib: compile_service, codegen, compiled_preflight, runtime_entry, inlining, native_compilation_admission, site_claims, optimiser, interprocedural, compilation_unit | 1101 ok | 1116 ok | 2 → contract tests restated (§1.4) |
| tcl-registry lib `native_string_operand` | 3/4 | 4/4 | 0 |
| tcl-vm `cmd_string_e2e` | 13/51 | 13/51 | 0 |
| tcl-vm `mathfunc_derivation_e2e` | 7/9 | 7/9 | 0 |
| tcl-vm `word_value_resolution_e2e` | 3/3 | 3/3 | 0 |
| tcl-debugger lib (400-deep test skipped) | 17/17 | 17/17 | 0 |
| tcl-engine-tclvm lib | 35/51 | 35/51 | 0 |
| tcl-spectcl lib | 302/310 | 304/310 | 0 |
| tcl-spectcl `codegen_stamps` (heavy matrix skipped) | 3/28 | 5/28 (pending re-run with §2/§3 fixes) | 0 |
| tcl-spectcl `pack_source_e2e`, `spec_corpus`, `golden_packs` | pending | pending | |

Notes on the rows:

- The tcl-compiler filter run aborts at `optimiser::propagation::tests::depth_exhaustion_blocks_dynamic_name_load_forwarding`
  in both modes (no `test result` line), so tests scheduled after it were not reported in either; the two sets are
  compared over what both ran.
- `codegen_stamps::every_shape_of_inlined_body_answers_as_its_definition_does_at_every_site` runs 2,898 VM
  evaluations; with the fix it gets past its first assertion and was still progressing after 20 min CPU in a debug
  build, so it is skipped in both modes.
- tcl-engine-tclvm `a_whitelisted_ensembles_subcommands_run_wherever_they_are_called` also needs `string map`.

## Resumed execution (2026-10-22)

Active checkout `/workspace/naming-migration-active/tcl-lsp`, GO from `d7224c706`. Results in this section are agent-reported, not guarded validation or admission. The original first-run fold probe is still pending the compiler-names build slot. The canonical owner file is `rust/tcl-compiler/src/const_subst.rs` (the outstanding brief's `optimiser/const_subst.rs` spelling was wrong). Decision `user-decisions-20261022T1317Z.json` authorises applying the world barrier only to the selected schema's declared world-effect chain; fallback invocation effects remain conservative for all other analyses and calls. No implementation change to that gate has yet been made.

A new sibling N17 control test is written but unrun: undeclared pure hook folds `good`; abstaining/no-hook/dynamic/unknown-child calls do not fold; the same successful hook with an explicit callback footprint does not erase the call; explicit EMPTY permits it; removing the host withholds it. The existing N17 test still requires `good` for both variable-reference spellings and both DSL releases. Temporary toggle removal uses the exact-text script with only its checkout root adapted. The resumed runner holds the global build lock, drains capped logs and retains executables younger than 60 minutes.

Native `string is/map/range/replace/insert/format` compilers and remaining frame-size work are parked unless a supported SpecTcl execution failure proves they are necessary. Their textual occurrence in a pack or hook is not such proof: a runtime handler can execute a call without a native compiler. The existing `STRLEN_BODY` uses `string is ascii`; the live-fold probe and full const-fold target will determine whether it actually fails there. Earlier `codegen_stamps` failures predominantly cannot find the original emitted procedure, so they do not establish a missing-string-compiler dependency.

## Remaining

1. Parked: `string is` (and `map`, `range`, `replace`, `insert`, `format`) native compilers — §5; unpark only for a demonstrated SpecTcl execution need.
2. Uncalled procedure bodies (Sol-47 §1d) are unchanged.
3. compile_service:: still fails 18 tests that failed at baseline: procedure targets and entered-source compiles
   without an entry (`compile_procedure_for_profile`, `compile_script_for_profile`). They are not whole-module
   documents; whether an entry-less *procedure* target is also a fresh interpreter is a separate decision.

## Gates

(filled at hand-back)
