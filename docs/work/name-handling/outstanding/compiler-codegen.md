# Outstanding work: compiler-codegen agent

Area: compile_service, codegen, compiled_preflight, tcl-spectcl install.rs, review N17.

Last updated: 2026-10-22T15:06Z (RESUMED; agent-reported, not validation)

Maintained by the compiler-codegen agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

Active checkout: `/workspace/naming-migration-active/tcl-lsp`, resumed from `d7224c706` under the coordinator's GO. The canonical const-fold owner path is `rust/tcl-compiler/src/const_subst.rs`; earlier `optimiser/const_subst.rs` references below are shorthand mistakes. Decision `user-decisions-20261022T1317Z.json` authorises the declared-footprint-only barrier fix. First run remains the existing fold probe before any fix or toggle removal. Builds source `/tmp/tcl-lsp-naming-env.sh` and hold `/tmp/tcl-lsp-naming-build.lock`; compiler-names has the first slot. No commits/push by this role. Adapt the old untoggle script's checkout root only, preserving exact-text assertions. General string compiler parity and frame-size work are parked unless a SpecTcl execution failure establishes need.

State at pause (13:10Z): no build or run of mine is running; my executables are deleted. Another agent's `cargo test --locked -p tcl-compiler --lib --no-run` was running and was left alone.

**Left in the tree on purpose (needed on resume):**
- TEMP toggle `ZZ_CCG_BASELINE` (each site marked `// TEMP measurement toggle (compiler-codegen), remove before hand-back.`) in: `rust/tcl-compiler/src/compile_service.rs`, `command_binding/compiled_preflight.rs` (also a `ZZ_CCG_PROVIDER` eprintln + `#[track_caller]` in `require_provider`), `command_binding/compiled_preflight/original_preparation.rs`, `codegen/control_flow.rs`, `optimiser/manager.rs`, `rust/tcl-spectcl/src/install.rs` (2 sites), `rust/tcl-registry/src/native_string_operand_compilation.rs`. Probe tests `zz_ccg_release_probe` and `zz_ccg_o129_probe` in `compile_service.rs`. With the variable unset, the tree runs the new behaviour.
- Probe `rust/tcl-spectcl/tests/zz_ccg_fold_probe.rs` (copy: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/zz_ccg_fold_probe.rs.txt`). Not yet run; its build was stopped at the pause.
- Removal: `python3 docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/untoggle.py` removes every toggle, the diagnostic, both probe tests and the probe file. It asserts each exact text, so it fails loudly if the text drifted.

**Exact next command (top item: const_fold / N17 live hook, release-blocking):**
`cd /home/user/tcl-lsp && df -h / && CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 cargo test -p tcl-spectcl --test zz_ccg_fold_probe -- --nocapture 2>&1 | grep -E "^error|ZZP|test result"; rm -f target/debug/deps/zz_ccg_fold_probe-*`
The probe compares O129 output for the pack registry (`registry_for_dialect_with_packs("tcl8.6")`) with the pack spec copied into `build_default`, copied with `world_effects = Some(EMPTY)`, and a pack that writes `world_effects none`.

**Leading hypothesis (from reading the code, not yet confirmed):** `const_subst.rs::fold_retained_at_depth` declines when `invocation.facts.effects.requires_world_barrier()`. A spec with no declared `world_effects` resolves to `EffectFootprint::conservative_unknown_invocation()` (callback reentrancy AnyInterpreter; `resolved_invocation.rs` ~2575). That is every pack command, so every live const_fold hook is gated off, even the shipped control and a trivial `fold good`. The gate came in with 944d156922 (2026-10-06).
- If the probe shows `D`/`world_effects none` fold and `B` does not, the fix is in `const_subst.rs`. Since it is not my file, I check `git status` first and ask if someone else has it modified. The fix: apply the barrier only to a *declared* footprint, so an authored const_fold callback on a spec/subcommand with no `world_effects` declaration is the author's claim for the call it replaces. Builtins with a declared callback footprint keep the gate. Two more possibilities: set this in the pack install, which over-claims for unfolded calls, so it is not preferred; or require packs to write `world_effects none`, which would break the documented 1.x meaning, so that is rejected.
- Then: pack_source_e2e `a_pack_file_folds_a_call_site_in_the_optimiser` should pass, and `rust/tcl-spectcl/tests/variable_reference_boundaries.rs::a_live_hook_folds_the_value_its_substitution_computes` (already written; DSL 1 and 2.0, `$arr($a:b)` and `$arr(${a}:b)`; expects O129 `good`) is the N17 regression. Also check `tcl-spec-hooks --test const_fold_e2e`.

**After that:** run untoggle.py, then `cargo check -p tcl-compiler -p tcl-spectcl -p tcl-registry --all-targets`, then cargo fmt on the changed files. Then one cargo command per target, deleting the executable in the same command: `-p tcl-compiler --lib compile_service::`, `codegen::native_failure`, `codegen::control_flow`; `-p tcl-spectcl --lib install::`; `--test variable_reference_boundaries`; `--test pack_source_e2e`; `--test codegen_stamps -- --test-threads=4 --skip every_shape_of_inlined_body`; `--test golden_packs`. Then the notes and the hand-back.

**Partial before/after results (same binary; base = `ZZ_CCG_BASELINE=1`; 0 pass→fail unless noted)**, lists in `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/results/`:
- tcl-compiler lib filters 1101 → 1116 ok. The 2 pass→fail are the old contract tests in `codegen/native_failure.rs`, rewritten to the user decision.
- compile_service:: 21/49 → 35/53.
- registry string 3/4 → 4/4; cmd_string_e2e 13/51 → 13/51; mathfunc 7/9 → 7/9; word_value 3/3 → 3/3.
- tcl-debugger lib 17/17 → 17/17; tcl-engine-tclvm lib 35/51 → 35/51.
- tcl-spectcl lib 302/310 → 304/310.
- pack_source_e2e 3/6 → 3/6. Failing in both: the O129 fold test, derived body, reference body.
- spec_corpus 3/5 → 3/5. Failing in both: the hostile-pack test and `every_shipped_tclspec…`.
- golden_packs base 3/3 ok (893 s); the after run was stopped by the pause, so not yet measured.
- codegen_stamps base 3/28 (the after run is due after the toggle is removed).
- Scripts: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/run_target.sh`, `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/chain2.sh` (scratch copies live in the session scratchpad and are not needed).

**Files I touched this session:** `rust/tcl-compiler/src/compile_service.rs`, `command_binding/compiled_preflight.rs`, `command_binding/compiled_preflight/original_preparation.rs`, `codegen/control_flow.rs`, `codegen/emitter/mod.rs`, `codegen/statements/native_scalar.rs`, `codegen/native_failure.rs`, `optimiser/manager.rs`; `rust/tcl-spectcl/src/install.rs`, `rust/tcl-spectcl/tests/variable_reference_boundaries.rs` (new), `rust/tcl-spectcl/tests/zz_ccg_fold_probe.rs` (TEMP); `rust/tcl-registry/src/native_string_operand_compilation.rs`, `native_compilation.rs`, `commands/tcl/string_.rs`; `runtime/rust/src/interp/native_body_artifact/native_string_operand.rs`; docs: this file, `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen-NOTES.md`, `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/`.

## In progress

- As of the `fabfda18b` coordinator snapshot: compiler-names first early-window attempt executed zero tests because tracked include inputs were missing; those inputs are restored and its first window is restarting. This role has not started cargo, no N17 verdict is claimed, and the original const-subst barrier remains unchanged until the fold probe measures it. New controls are written/formatted only.
- Release lint work added by the coordinator: `/tmp/naming-clippy-compiler-codegen.json` is an external `aeecf397` Rust 1.99.1 inventory, 54 diagnostics across 17 owned paths. Current hunks are inspected; mechanical fixes and helper splits are being prepared in `/tmp/ccg-lint-{base,work}`, with no canonical product edits. A shared `registry_invocation::InvocationMetadataSelection { Standalone, Retained(context) }` (outer absence still terminal refusal) is coordinated with compiler: compiler owns the API plus SSA/static_loops consumers, this role owns method_barrier and emitter consumers. No duplicate alias/enum will ship. The coordinator explicitly postpones canonical lint/API application until the unchanged N17 probe and functional chain are measured.

- const_fold live hooks (release-blocking, coordinator's top item): no live pack const_fold hook folds in `optimise_raw`. The world-barrier hypothesis and the probe are in Resume.

## Outstanding (in priority order)

1. const_fold fix plus N17 live-hook regression (see Resume).
2. Remove the TEMP toggle, the probe tests and the probe file (untoggle.py), then fmt/clippy/check, then the final runs listed in Resume (golden_packs after, codegen_stamps after).
3. Notes and hand-back (`docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen-NOTES.md`, draft present, with Measurements rows still pending).
4. `string is` native compiler (C TclCompileStringIsCmd; 8.6 / 9.0 / 9.1 variants). Not started. Also `string map/range/replace/insert/format` (the engine's `a_whitelisted_ensembles…` needs `string map`).
5. CompilerTraversal frame: re-measure against the 24 KB budget after the last split (`compiled_command`, `boxed_command_tokens`).

## Blocked (on whom, and what is needed)

- Waiting for explicit compiler-names early-window release before measuring the unchanged N17 fold probe; its dependency/compiler build is running, and no build of this role has started. No new test verdict is claimed. New N17 negative controls are written and formatted; canonical const_subst.rs barrier is unchanged. Rebase pause/CLEAR completed to work41e547bcb. Live logs are configured under /tmp and copied into evidence only on completion. Untoggle script root is adapted, but it is not run: require_provider has one additional TEMP comment absent from its expected text, to remove manually after measurement while preserving all script assertions.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Contracts (1)–(3): entry-less whole-module compiles = fresh interpreter (BytecodeObject + resolved input; Direct fallback when the catalogue lacks a descriptor); compiled_preflight entry-less provider rule; fresh-world expression emission; profile-less → plain_tcl's engine, manifest restamped. compile_service:: 21/49 → 35/53 (0 pass→fail at that point).
- tcl-engine-tclvm lib 35/51 → 35/51 (0 pass→fail); its `a_whitelisted_ensembles_subcommands_run_wherever_they_are_called` also needs `string map` (no compiler yet).
- tcl-spectcl install.rs: pack rows get derived native compilers (override keeps shipped; new = NoHook; admitted alias_of = target's) + 2 tests.
- `string cat` native compiler (registry + codegen + runtime/rust); registry filter 4/4 (base 3/4).
- `dict update`/`dict with` release_compiled.
- Measured before → after (same binary, toggle): codegen_stamps 3/28 → 5/28 (heavy matrix skipped), tcl-vm word_value 3/3 → 3/3, mathfunc 7/9 → 7/9, cmd_string 13/51 → 13/51, tcl-debugger lib 17/17 → 17/17; tcl-compiler lib filters (compile_service, codegen, compiled_preflight, runtime_entry, inlining, optimiser, interprocedural, compilation_unit) 1101 → 1116 ok, 17 gains; 2 pass→fail were the old "missing entry ⇒ provider required" contract tests in codegen/native_failure.rs, rewritten to the user decision (unknown entry keeps the obligation; entry-less fresh compile presents the 8.4 compiler's own rejection, equal to a captured fresh entry's).

## Coordinator note (2026-10-22T13:25Z)

`optimiser/const_subst.rs` is assigned to compiler-codegen (`user-decisions-20261022T1317Z.json`; the user asked the coordinator to choose). Confirm the hypothesis with the fold probe first, then apply the proposed fix: the world barrier applies only to a declared `world_effects` footprint. Add a regression test showing that an undeclared-effect pack hook folds only the call it replaces, and that every other call stays fail-closed.
