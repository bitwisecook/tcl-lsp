# Outstanding work: compiler-codegen agent

Area: compile_service, codegen, compiled_preflight, tcl-spectcl install.rs, review N17.

Last updated: 2026-10-22T12:40Z

Maintained by the compiler-codegen agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

Where to pick up (if this agent stops):

1. If `grep -rn ZZ_CCG rust runtime --include=*.rs` finds anything, the TEMP measurement toggle is still in. Finish any measurement still wanted, then run `python3 docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/untoggle.py` (asserts each exact text, so it fails loudly if the text drifted), delete `rust/tcl-spectcl/tests/zz_ccg_fold_probe.rs`, then `cargo check -p tcl-compiler -p tcl-spectcl -p tcl-registry --all-targets` and `cargo fmt` on the changed files.
2. const_fold hook (N17 live hook; pack_source_e2e `a_pack_file_folds_a_call_site_in_the_optimiser`): run the probe, which prints the registry profile, the spec's thunk result, `resolve_call`, and `optimise_raw` over the pack registry versus `build_default` with the pack spec inserted:
   `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p tcl-spectcl --test zz_ccg_fold_probe -- --nocapture 2>&1 | grep ZZP; rm -f target/debug/deps/zz_ccg_fold_probe-*`
   The gates on the O129 path are in `rust/tcl-compiler/src/const_subst.rs::fold_retained_at_depth` (registry-backed target, metadata context, `invocation.dialect`, world barrier, `get_for_surface`).
3. Measurements: `run_target.sh NAME "cargo args" [test args]` builds once, runs with `ZZ_CCG_BASELINE=1` (old behaviour) and without (new), deletes the executable, and prints pass→fail. `chain2.sh` lists every target measured. Still to run after the toggle is removed: `cargo test -p tcl-spectcl --test codegen_stamps -- --test-threads=4 --skip every_shape_of_inlined_body` (after-only; base 3/28 is in results), `-p tcl-spectcl --test variable_reference_boundaries`, `-p tcl-compiler --lib compile_service::`, `codegen::native_failure`, `codegen::control_flow`, `-p tcl-spectcl --lib install::`.
4. Evidence in the repo: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/` (scripts; `results/chain2.log` = summary lines; `results/*-{base,after}-ok.txt` = passing-test lists per target, diffable with `comm -23`). Notes: `../compiler-codegen-NOTES.md` beside that folder.

## In progress

- Before/after measurement chain (TEMP env toggle `ZZ_CCG_BASELINE` in the tree, marked; removed right after): tcl-spectcl lib 302/310 → 304/310 (0 pass→fail), pack_source_e2e 3/6 → 3/6 (0 pass→fail), spec_corpus 3/5 → 3/5 (0 pass→fail); golden_packs running; codegen_stamps re-run after the toggle is removed.
- const_fold hook path (next, per coordinator; N17 live-hook acceptance depends on it): fixes already in the tree — `optimise_raw(.., None)` over a profile-projected registry analyses for that profile (it was dialect-blind and folded nothing, even `string length`), and pack rows now carry native compiler descriptors (install.rs). Regression `variable_reference_boundaries.rs::a_live_hook_folds_the_value_its_substitution_computes` (DSL 1/2.0 × `$arr($a:b)` / braced control) awaits a run. pack_source_e2e `a_pack_file_folds_a_call_site_in_the_optimiser` still gets no O129 with both fixes in, so a further gate declines; a TEMP probe (`rust/tcl-spectcl/tests/zz_ccg_fold_probe.rs`, removed with the toggle) traces registry profile → spec thunk → resolve_call → optimise_raw once the chain frees the CPU.

## Outstanding (in priority order)

1. N17 (review naming-migration-review-20261022): regressions written — `compile_service::tests::a_variable_reference_ends_where_the_release_scanner_ends_it` (VM run, 8.4–9.1, top level and proc) and `rust/tcl-spectcl/tests/variable_reference_boundaries.rs` (template ×DSL 1/2.0 ×direct/helper ×fast path, braced control, lone colon, colon runs; live const_fold hook → O129 `good`). Current-tree code already decomposes array keys with the real lexer (`push_array_key` → `ExecutablePartArena`; `emit_variable_reference` → `is_whole_variable_reference`); run the regressions to confirm, fix if any fails.
2. Remove the TEMP toggle and the two `zz_ccg_*` probe tests (script ready); cargo fmt on my files; clippy on changed lines; re-run compile_service::, codegen::native_failure, tcl-spectcl install::, the new spectcl test file, codegen_stamps.
3. Notes + hand-back (`docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen-NOTES.md`, draft present).
4. `string is` native compiler (C TclCompileStringIsCmd; three release variants 8.6 / 9.0 / 9.1 with different opcodes: IS_EMPTY, SWAP, `dict` class from 9.0, catch ranges for list/dict). Not started; multi-crate (registry grammar + plan + preflight + codegen + runtime/rust arm).
5. O129 / const_fold: `optimise_raw(src, registry, None)` analysed dialect-blind and folded nothing (even `string length`); now uses the registry's own profile. Pack rows also needed native compiler descriptors (install.rs). Confirm pack_source_e2e / spec_corpus hostile-pack after the chain.
6. CompilerTraversal frame: `command` split into plan (`command_visit`, 7.9 KB) + small descent frames; per-level path script 4.7 KB + command 1.6 KB + compiled_command (was 6.1 KB, now shrunk further, re-measure) + visit/bodies. Re-measure against the 24 KB budget.

## Blocked (on whom, and what is needed)

- Nothing blocked now. Disk is the throttle (builds only above 2 GB).

## Done since the last update (with evidence: test counts, commit or snapshot)

- Contracts (1)–(3): entry-less whole-module compiles = fresh interpreter (BytecodeObject + resolved input; Direct fallback when the catalogue lacks a descriptor); compiled_preflight entry-less provider rule; fresh-world expression emission; profile-less → plain_tcl's engine, manifest restamped. compile_service:: 21/49 → 35/53 (0 pass→fail at that point).
- tcl-engine-tclvm lib 35/51 → 35/51 (0 pass→fail); its `a_whitelisted_ensembles_subcommands_run_wherever_they_are_called` also needs `string map` (no compiler yet).
- tcl-spectcl install.rs: pack rows get derived native compilers (override keeps shipped; new = NoHook; admitted alias_of = target's) + 2 tests.
- `string cat` native compiler (registry + codegen + runtime/rust); registry filter 4/4 (base 3/4).
- `dict update`/`dict with` release_compiled.
- Measured before → after (same binary, toggle): codegen_stamps 3/28 → 5/28 (heavy matrix skipped), tcl-vm word_value 3/3 → 3/3, mathfunc 7/9 → 7/9, cmd_string 13/51 → 13/51, tcl-debugger lib 17/17 → 17/17; tcl-compiler lib filters (compile_service, codegen, compiled_preflight, runtime_entry, inlining, optimiser, interprocedural, compilation_unit) 1101 → 1116 ok, 17 gains; 2 pass→fail were the old "missing entry ⇒ provider required" contract tests in codegen/native_failure.rs, rewritten to the user decision (unknown entry keeps the obligation; entry-less fresh compile presents the 8.4 compiler's own rejection, equal to a captured fresh entry's).
