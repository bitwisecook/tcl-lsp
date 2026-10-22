# Outstanding work: compiler-codegen agent

Area: compile_service, codegen, compiled_preflight, tcl-spectcl install.rs, review N17.

Last updated: 2026-10-22T16:54Z (RESUMED; agent-reported, not validation)

Maintained by the compiler-codegen agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

Active checkout: `/workspace/naming-migration-active/tcl-lsp`. Do not use the old `/workspace/tcl-lsp` or transfer clone. Source `/tmp/tcl-lsp-naming-env.sh` (Rust 1.99.1, jobs 2, no incremental/debug info, all five Tcl wrappers and Jim) and hold `/tmp/tcl-lsp-naming-build.lock` over each entire build/run/receipt operation. Stop below 2 GB free. Keep live capped logs under `/tmp`, copy only completed evidence, and never remove executables younger than 60 minutes. No commits/push by this role.

**Currently running:** exec session `96934`, `n17_public_axes`, ONE combined public diagnostic in our temporary tracked `zz_ccg_fold_probe` target. It has no acceptance assertions. Original probe, combined source and external helper are preserved as `.rs.txt` evidence; helper SHA `d91e79ba...` is separately tracked because its `/tmp` include is outside the canonical manifest. No further Cargo target before stable-window handback.

**Cleanup hold:** finish diagnostic/manifest, but WAIT coordinator's cycle-commit CLEAR before canonical untoggle; root's 16:55 WIP commit must not stage a half-cleaned tree. Publication may proceed separately. Root will stage only the removed tracked probe after cleanup.

**Latest functional results:** boundary target 4/4, zero warnings (compile 25.19 s / execution 214.87 s). Full pack_source_e2e **3/6**, zero warnings (compile 12.76 s / execution 39.86 s): explicit strlen O129 `[]` vs `[5]`; derived expr `None` vs `42`; derived tcl8.6 cat `None` vs `xy`. Timing, loader and no-host controls pass. Both completed runs retain exact source equality (35,072 files / zero missing, SHA `1bbe2857b2d87a8f526d5420341bfd7063e634942b167c2913471a426e7ef506`). These three failures remain release blockers, not explained or waived. Full const_fold_e2e also finished **3/6**, zero compiler warnings (build 18m44s / execution 28.37 s; alternate runtime-engine feature closure). Dynamic/no-host/abstention pass; pipeline emits no O129, versioned tcl8.6 emits none, and direct pack folder returns None on the first empty input where native gives 0. Later direct inputs/profile rows were not reached after those assertions. Its source equality is the same 35,072/zero-missing digest; the first network-blocked attempt remains a separate 0-test receipt.

**Canonical const-fold file:** `rust/tcl-compiler/src/const_subst.rs` (earlier `optimiser/const_subst.rs` spelling was wrong). Decision `user-decisions-20261022T1317Z.json` grants this file and the declared-footprint-only barrier. The unchanged probe confirmed the hypothesis, then the narrow fix was applied: only a selected declared world-effect chain triggers the barrier; missing selected schema/realm still refuses. No other product source change is mixed into it. Frozen baseline manifest vs first after-fix manifest differs at exactly this one path (`n17_narrow_fix_source_delta.json`).

**Next:** finish ONE combined diagnostic and source guard, verify external helper hash unchanged, then exact cleanup after root CLEAR and stable-window handback even if functional failures remain. Explicit-hook diagnostic crosses identical installed strlen body × caller formals `{}`/`{x}` × undeclared/EMPTY/WRITE, with separate fresh-host direct abcde/empty hook/error observations and public binding/normal-completion/catalogue-schema facts. The VM role's derived-body helper separately prints direct Evaluate dispatch, public SCCP routes/all known SSA versions and original procedure counts. Keep original E2E tests unchanged. Root then integrates published external updates and schedules R1/C6/shared lint; further functional fixes can be prepared off-tree meanwhile.

**Then exact temporary cleanup:** `python3 docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/untoggle.py`. Its checkout root is adapted to this active path. A coordinator-approved exact-pattern correction removes a redundant literal `// ` before T (already `// TEMP`); every assertion and intended output is preserved. Off-tree dry-copy receipt `untoggle_dry_copy.json` records all exact assertions passing and 7/7 syntax parses, plus expected output hashes. Canonical cleanup has not run. Known drift: `require_provider` contains one extra TEMP measurement comment absent from the script's expected text; remove only that comment manually immediately before running the script, after measurements. Back up/review only owned files; do not touch peer work. The script removes source probes/toggles, not test executables.

Still deliberately present until that cleanup: `ZZ_CCG_BASELINE` in compile_service, compiled_preflight, original_preparation, codegen/control_flow, optimiser/manager, tcl-spectcl/install (two sites), registry/native_string_operand_compilation; `ZZ_CCG_PROVIDER` diagnostic + track_caller; compile_service `zz_ccg_release_probe` / `zz_ccg_o129_probe`; tracked `rust/tcl-spectcl/tests/zz_ccg_fold_probe.rs` (evidence copy under `compiler-codegen/zz_ccg_fold_probe.rs.txt`). Variables remain unset for the functional chain.

**After functional measurements/cleanup:** root will coordinate canonical lint/API changes before broad compiler-lib checks, avoiding another unnecessary full test artefact rebuild. Shadow lint preparation is in `/tmp/ccg-lint-{base,work}`, draft `/tmp/ccg-lint-prepared.patch`; do not copy shadow files wholesale because they retain pre-untoggle text. Then required fmt/check/clippy and focused compile_service/native_failure/control_flow/install/codegen_stamps/golden targets, notes and hand-back. Never execute the whole compiler library.

All Rust AND test files are now frozen for this functional chain except this role's authorised N17 fix/cleanup. Docs-only coordinator snapshots may advance HEAD without rewriting source files. No peer production integration or lint/API edits until this chain completes.

## In progress

- Frozen baseline `n17_probe_before_frozen`: exit 0, diagnostic harness 1/1, cached build 1.17 s / execution 36.68 s. Full source equality passes: 35,072 files, zero missing, `1dde2936471965040575de1e2b63a42fe80ed5f61a34be76e852960cd6a7af54` before/after; HEAD `4c5718d36`. Direct hook returns `5`, undeclared-effect pack call emits no O129, otherwise identical EMPTY clone under tcl8.6 emits `5`, authored `world_effects none` emits `5`; builtin control emits `5`. This is a reproduced defect with passing diagnostic harness, not N17 acceptance.
- First baseline `n17_probe_before` also reproduced it, but its full source hash drifted (`67e38c9e...` → `1dde2936...`) due three Server E2E files outside this target's compiled closure. It remains unguarded observational evidence. One warm-repeat receipt-writer error occurred before Cargo (0 tests), was fixed off-tree and recorded separately. No result is omitted.
- Narrow const-subst fix is applied; first after-fix target did not compile (0 tests), with exact before/after source equality `918e30ee02650850cdfc9177d031637562a1626031820e89490c958e795288b9`. The owned-registry test-construction correction is ready for a distinct rerun. The new sibling controls cover undeclared pure fold, abstention, no hook, dynamic/unknown-child arguments, explicit callback barrier, EMPTY and missing host; all pass in the corrected full target (4/4 total). The product fix remains unchanged; only the control construction was adapted from shared Arc to an owned same-profile projection for effect mutations, retaining overlay/origin metadata by explicit API implementation. No assertions were weakened.
- Runner `/tmp/ccg-run-target.py` saves per-file gzip source manifests, startup receipts, capped logs, exits/HEAD/time/log digest. These are agent-reported runs, not independently guarded validation or audit-unit admission. No executable cleanup.
- Lint assignment: external `aeecf397` Rust 1.99.1 inventory `/tmp/naming-clippy-compiler-codegen.json`, 54 diagnostics / 17 owned paths. Shadow draft also includes the shared metadata API's emitter consumer (18 files). Rustfmt parses the shadows; no compile/Clippy verdict. Compiler owns shared `InvocationMetadataSelection { Standalone, Retained(context) }` API plus SSA/static_loops consumers; this role owns method_barrier/emitter consumers. Outer absence remains terminal refusal; no duplicate private enum will ship.
- Static lead only: `codegen_stamps.rs::compile` line 448 bypasses BytecodeCompileService and uses raw profile-less lowering/CFG/codegen. Earlier missing-procedure-provenance failures may reflect absent fresh source metadata there. Inspect its production owner after N17 before any fixture/assertion change or unparked string compiler.

## Outstanding (in priority order)

1. const_fold fix plus N17 live-hook regression (see Resume).
2. Remove the TEMP toggle, the probe tests and the probe file (untoggle.py), then fmt/clippy/check, then the final runs listed in Resume (golden_packs after, codegen_stamps after).
3. Notes and hand-back (`docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen-NOTES.md`, draft present, with Measurements rows still pending).
4. Assigned compiler-codegen/preflight/optimiser strict Clippy errors; apply coordinated batch after N17 functional measurements, then compile/lint and required focused checks. No new allowances or assertion weakening.

## Parked unless needed by SpecTcl

- Remaining `string is/map/range/replace/insert/format` native compilers. Textual use by a hook is not evidence that a native compiler is required: its runtime handler may execute it correctly. No such native-compiler dependency is established by this resumed role yet.
- CompilerTraversal frame re-measurement against the 24 KB budget after the last split (`compiled_command`, `boxed_command_tokens`), unless a supported LSP/SpecTcl path establishes the need.

## Blocked (on whom, and what is needed)

- No external setup blocker. Diagnostic is running, then canonical cleanup waits for root cycle-commit CLEAR. Full pack_source_e2e retains three supported-path release blockers requiring the planned diagnostic. No general native-string compiler dependency is proved. Canonical lint/API work waits for completion of the authorised functional chain under root's source freeze. Other required checks remain unexecuted, not passed.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Contracts (1)–(3): entry-less whole-module compiles = fresh interpreter (BytecodeObject + resolved input; Direct fallback when the catalogue lacks a descriptor); compiled_preflight entry-less provider rule; fresh-world expression emission; profile-less → plain_tcl's engine, manifest restamped. compile_service:: 21/49 → 35/53 (0 pass→fail at that point).
- tcl-engine-tclvm lib 35/51 → 35/51 (0 pass→fail); its `a_whitelisted_ensembles_subcommands_run_wherever_they_are_called` also needs `string map` (no compiler yet).
- tcl-spectcl install.rs: pack rows get derived native compilers (override keeps shipped; new = NoHook; admitted alias_of = target's) + 2 tests.
- `string cat` native compiler (registry + codegen + runtime/rust); registry filter 4/4 (base 3/4).
- `dict update`/`dict with` release_compiled.
- Measured before → after (same binary, toggle): codegen_stamps 3/28 → 5/28 (heavy matrix skipped), tcl-vm word_value 3/3 → 3/3, mathfunc 7/9 → 7/9, cmd_string 13/51 → 13/51, tcl-debugger lib 17/17 → 17/17; tcl-compiler lib filters (compile_service, codegen, compiled_preflight, runtime_entry, inlining, optimiser, interprocedural, compilation_unit) 1101 → 1116 ok, 17 gains; 2 pass→fail were the old "missing entry ⇒ provider required" contract tests in codegen/native_failure.rs, rewritten to the user decision (unknown entry keeps the obligation; entry-less fresh compile presents the 8.4 compiler's own rejection, equal to a captured fresh entry's).

## Coordinator note (2026-10-22T13:25Z)

`optimiser/const_subst.rs` is assigned to compiler-codegen (`user-decisions-20261022T1317Z.json`; the user asked the coordinator to choose). Confirm the hypothesis with the fold probe first, then apply the proposed fix: the world barrier applies only to a declared `world_effects` footprint. Add a regression test showing that an undeclared-effect pack hook folds only the call it replaces, and that every other call stays fail-closed.
