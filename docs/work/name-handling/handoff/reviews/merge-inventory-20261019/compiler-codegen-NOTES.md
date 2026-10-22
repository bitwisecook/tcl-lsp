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
| `rust/tcl-compiler/src/const_subst.rs` | granted by decision 2026-10-22T13:17Z | declared-world-footprint-only barrier measured after frozen baseline; functional checks in progress |
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
`range`, `replace`, `insert`, `format` likewise remain (prior compiler inventory).

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
`string cat` Unknown, no pack-row compilers, dialect-blind `optimise_raw`); "after" without it. The toggle, inline probe tests and tracked integration probe were removed by exact authorised cleanup on 2026-10-22T16:56Z, after the resumed measurements below. `TCL_LSP_REQUIRE_TCLSH=9.0,8.6`, `TCL_LSP_JIMSH`.

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

Active checkout `/workspace/naming-migration-active/tcl-lsp`. Results here are agent-reported, not guarded validation or admission. The canonical owner file is `rust/tcl-compiler/src/const_subst.rs` (the outstanding brief's `optimiser/const_subst.rs` spelling was wrong). Decision `user-decisions-20261022T1317Z.json` authorises applying the world barrier only to the selected schema's declared world-effect chain; fallback invocation effects remain conservative for all other analyses and calls.

The unchanged probe reproduced the missing O129 despite a successful direct hook. Its identical-spec EMPTY control and authored `world_effects none` control emit O129 `5`; undeclared effects prevent the ordinary pack call. First run passed the observational harness 1/1 but had three unrelated Server E2E edits outside this target's compiled closure, so it is unguarded scoped evidence. After the coordinator froze all Rust/tests, `n17_probe_before_frozen` passed the same diagnostic 1/1 with identical full manifests (35,072 files, zero missing, SHA `1dde2936471965040575de1e2b63a42fe80ed5f61a34be76e852960cd6a7af54`). One receipt-writer setup failure before Cargo (0 tests) is preserved separately. These diagnostic harness successes reproduce a defect and are not N17 acceptance.

The product fix is one const-subst hunk: consult the selected semantic schema when effects require a barrier, and refuse only when its footprint is declared. Missing selected schema/realm still refuses; other depth, handler, proof, argument and abstention gates are unchanged. The first after-fix source delta is exactly this file (`n17_narrow_fix_source_delta.json`). Its full target attempt failed at compilation (0 tests): the new control tried to insert through the installer's shared Arc, yielding two E0596 errors plus unused-mut. That receipt remains recorded with source equality.

The narrow test setup adaptation keeps initial production-Arc pure/no-hook/abstention/dynamic/unknown-child controls, then obtains an owned same-profile projection for declared-effect mutations. `CommandRegistry::project_for_profile` explicitly retains authored overlays, their generation, pack origins and reference texts (registry.rs:2207); no cloning assumption is used. All assertions are unchanged. Manifest comparison now proves exactly the const-subst fix plus this test-construction adaptation since frozen baseline (`n17_control_construction_source_delta.json`). Rerun `n17_variable_boundaries_after_owned_registry` passed 4/4 with zero warnings (compile 25.19 s, execution 214.87 s). Full source equality passed: 35,072 files / zero missing, SHA `1bbe2857b2d87a8f526d5420341bfd7063e634942b167c2913471a426e7ef506`; docs-only publication advanced HEAD during the run. Full pack_source_e2e finished 3/6 (compile 12.76 s / execution 39.86 s), with identical source digest and zero warnings: explicit strlen O129 is absent, derived expr42 and tcl8.6 catxy return None; timing/loader/no-host controls pass. These remain release blockers. const_fold_e2e first failed before tests on a restricted-network crc32fast download; the authorised network-enabled retry finished 3/6 after building the alternate runtime-engine feature closure (18m44s compile / 28.37s execution; no compiler warnings). Dynamic/no-host/abstention pass. Pipeline O129 and versioned tcl8.6 fail; direct pack folder already returns None on empty input versus native 0. Subsequent inputs/profiles were not reached. Full source equality again passes the same 35,072-file digest. The single combined public diagnostic completed observationally 1/1 (13.01s compile / 118.06s execution): explicit ascii-guarded hook direct abcde/empty refuses `native compiler preflight provider required`; both caller forms retain the expected procedure/metadata/handler proof, and EMPTY does not repair body admission. Declared WRITE blocks before host dispatch. Derived expr42/catxy direct Evaluate succeeds for default/EMPTY, while all eight CU rows retain ::p and trust but only v1 Overdefined with no route explanations. These are separate provider versus pre-route-analysis blockers. Canonical 35,072-file equality passes SHA9708d6e3...; the /tmp helper separately hashes d91e79ba... before/after.

Completed evidence: `compiler-codegen/results/resumed-20261022/`, with per-run command/exit/time/source receipts, capped logs and per-file gzip manifests. Temporary toggle removal is complete after measurements and the coordinator's cycle-commit CLEAR. The exact-text script has its checkout root adapted plus one coordinator-approved expected-pattern correction (redundant literal comment prefix); all assertion guards/output remain. Dry-copy exact assertions and seven Rust syntax parses pass (`untoggle_dry_copy.json`). The documented extra TEMP comment was removed immediately before canonical cleanup. Actual seven output hashes match the dry copy; full delta is seven toggle removals plus our tracked probe deletion, preserving the const-subst fix and regression assertions. Seven syntax parses emitted to stdout only; no executables, peer probes or broad formatting were touched. See n17_cleanup.json; root stages the tracked deletion. Final-source build/lint/gate results remain pending. No executable cleanup is performed. Canonical lint/API edits remain off-tree until root schedules integration after R1/C6. The authentic entry-capture/provider path is installed already: Engine creates a native C9.0 VM and installs BytecodeCompileService; activation supplies an authentic VM entry. Static tracing locates a missing typed registry descriptor for the real private ::tcl::string::is worker, yielding Unknown and the preserved preflight refusal. There is no synthetic provider setter/bypass repair. Exact non-strict compile-known ascii needs a typed >=8.6 descriptor and marked STR_CLASS path with original Unicode and native Boolean constants. VM C8.6/9.0/9.1 oracle controls pass; its extra old8.4 NUL failure stays recorded. Proposed11-path patch and7 new controls remain OFF-TREE/unrun. Copied composition with shared Selection/names56, own18 and VM47 drafts applies16 patches to133 unique files fuzz0, with canonical byte hashes unchanged. See ccg-next-batch-manifest.json. No same-body post-descriptor pass or final-source gate is claimed yet.

Only the exact non-strict compile-known ASCII `string is` branch is now required by the demonstrated supported SpecTcl refusal. Other `string is` forms plus `map/range/replace/insert/format` compilers and remaining frame-size work stay parked unless a supported execution failure proves them necessary. Their textual occurrence in a pack or hook is not such proof: a runtime handler can execute a call without a native compiler. The existing `STRLEN_BODY` uses `string is ascii`; direct host measurements and the typed private registration trace now establish that exact dependency. Earlier `codegen_stamps` failures predominantly cannot find the original emitted procedure, so they do not establish a missing-string-compiler dependency.

## Remaining

1. Exact non-strict compile-known ASCII StringIs draft is required and awaits coordinated apply/typecheck/acceptance. Other StringIs forms and `map/range/replace/insert/format` stay parked.
2. Uncalled procedure bodies (prior compiler inventory §1d) are unchanged.
3. compile_service:: still fails 18 tests that failed at baseline: procedure targets and entered-source compiles
   without an entry (`compile_procedure_for_profile`, `compile_script_for_profile`). They are not whole-module
   documents; whether an entry-less *procedure* target is also a fresh interpreter is a separate decision.

## Gates

(filled at hand-back)
