# Outstanding work: compiler-codegen agent

Area: compile_service, codegen, compiled_preflight, tcl-spectcl install.rs, review N17.

Last updated: 2026-10-22T16:57Z (RESUMED; agent-reported, not validation)

Maintained by the compiler-codegen agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

Active checkout: `/workspace/naming-migration-active/tcl-lsp`. Do not use the old or transfer clones. Source `/tmp/tcl-lsp-naming-env.sh`; Rust 1.99.1/jobs 2/no incremental/debug info/all five Tcl wrappers + Jim. Global `/tmp/tcl-lsp-naming-build.lock` covers each full build/run/receipt. Stop below 2 GB free; no executable cleanup younger than 60 minutes, no Cargo/rustc target deletion. No commits, pushes or Git staging by this role.

**Stable source handback, 16:57Z:** the focused chain and exact cleanup are COMPLETE. No Cargo or lint/API integration is running or queued. Root may stage ONLY this role's tracked probe deletion (`rust/tcl-spectcl/tests/zz_ccg_fold_probe.rs`), checkpoint/rebase/integrate its published updates, then schedule R1/C6/shared lint. This role makes no canonical Rust/test edits until root GO. Read-only/off-tree investigation may continue.

**Product fix retained:** canonical `rust/tcl-compiler/src/const_subst.rs` (earlier shorthand `optimiser/const_subst.rs` was wrong), explicitly granted by `user-decisions-20261022T1317Z.json`. Its world barrier now applies only to the selected schema's DECLARED footprint; unavailable selected realm/schema refuses, and other call/depth/handler/argument/abstention gates remain. Frozen before-probe source equality held; the first after-fix source delta was exactly this file. A later control-construction adaptation was test-only (shared Arc controls first, owned same-profile projection for effect mutation), with every assertion unchanged.

**Functional measurements (agent-reported, not guarded validation):**

| Target | Result | Compile / execution | Source equality |
|---|---|---|---|
| Frozen unchanged fold probe | diagnostic 1/1; reproduced missing O129 | 1.17 s / 36.68 s | 35,072 files, zero missing; `1dde2936...` |
| variable_reference_boundaries | **4/4**, zero warnings | 25.19 s / 214.87 s | 35,072 / zero missing; `1bbe2857...` |
| pack_source_e2e | **3/6**, zero warnings | 12.76 s / 39.86 s | same `1bbe2857...` |
| const_fold_e2e | **3/6**, zero warnings | 18m44s / 28.37 s (alternate runtime-engine feature closure) | same `1bbe2857...` |
| ONE combined public diagnostic | observational 1/1; no acceptance assertions | 13.01 s / 118.06 s | 35,072 / zero missing; `9708d6e3...`; external helper before/after separately `d91e79ba...` |

All completed receipts/logs/manifests are under `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/results/resumed-20261022/`. Preserve initial unguarded baseline (three Server E2E test edits outside target closure), pre-Cargo receipt-writer failure, new-control Arc compile failure (0 tests), and restricted-network missing-crate attempt (0 tests) as distinct evidence. None is silently omitted or counted as a functional pass. No pass-to-fail claim across unmeasured broader suites.

**Release blockers, now separated:**

- Explicit ascii-guarded strlen hook: DIRECT abcde/empty with/without 8.6 refuses with `execution refused: native compiler preflight provider required`, no crash. Both caller formal variants retain `::f`, accepted metadata, RuleLoader realm, registry-backed execution/handler and normal arguments. Undeclared/EMPTY both reach host and refuse; declared WRITE blocks before the host (errors empty). Existing simpler hook/boundary controls succeed. Preserve the provider guard; no bypass/toggle fix. Trace authentic native compiler-provider installation through compile_service/engine seam with VM role off-tree; a positive same-body provider-installed control is still unmeasured. No particular native-string compiler is proved necessary yet.
- Derived expr42/tcl9 and catxy/tcl8.6: fresh direct Evaluate succeeds (42/xy) with and without EMPTY, slots stay available, no host errors/crashes. All eight analysis rows retain `::p`, trusted heads, only known version 1 Overdefined, empty SCCP route explanations. Missing original publication, caller-formals mismatch, SSA-version mismatch and VM transport do not explain these fixtures. Compiler role is informed; SCCP/ConstOps pre-route admission remains to trace after root's R1/C6 priority.

**Exact cleanup completed:** root-authorised dry-copy maintenance removed a redundant literal comment prefix in untoggle.py's exact expected pattern; every assertion/output was preserved. The documented extra TEMP comment in require_provider was removed immediately before the script. Actual cleanup's seven output hashes exactly match dry-copy expectations, 7/7 Rust syntax parses pass with stdout-only emission (no formatter writes), and full per-file delta is exactly seven owned toggle removals plus our own probe deletion. `const_subst.rs` stays `c710e35b...`; boundary regression stays `d67c8bad...`. No executables or peer probes were removed. `n17_cleanup.json` and `n17_cleanup-source-after.json.gz` record the distinct unbuilt post-cleanup state; the one missing tracked path is the intentional probe deletion awaiting root staging, not an admission claim. Backups: `/tmp/ccg-cleanup-backup-20261022T165625Z`.

**Off-tree lint preparation:** external old-source inventory `/tmp/naming-clippy-compiler-codegen.json` assigns 54 diagnostics / 17 owned paths. `/tmp/ccg-lint-post-cleanup.patch` covers 18 paths including emitter's shared-API consumer, has no toggle/probe hunks, parses via rustfmt and dry-applies fuzz 0 to copied current source plus exact cleanup. NO compile/Clippy verdict. Compiler owns shared `InvocationMetadataSelection { Standalone, Retained(context) }` API plus SSA/static_loops consumers; this role owns method_barrier/emitter consumers. Outer absence remains refusal; no duplicate local alias. Apply only after root GO and reconcile current source; never overwrite with pre-cleanup shadows.

## Outstanding (in priority order)

1. Off-tree authentic provider seam diagnosis with VM; keep exact host refusal and positive/negative controls. Root's R1/C6 work proceeds independently.
2. Compiler-owned derived SCCP/ConstOps pre-route refusal; coordinate ownership with root/compiler, preserve assertions.
3. Coordinated 54-diagnostic lint/API batch after functional attribution, then fmt/check/strict Clippy with no new allowances. Current canonical source is unbuilt after cleanup.
4. Required focused compiler lib filters compile_service/native_failure/control_flow (never whole lib); install lib filter; codegen_stamps skipping the massive matrix; golden_packs; required check/fmt/clippy/xtask gates. These remain unexecuted for final source, not passed.
5. Final notes/hand-back after unresolved functional/gate results. Static `codegen_stamps::compile` raw profile-less lowering/provenance lead remains only a lead; no fixture/assertion changes from it.

## Blocked

- Canonical changes/builds wait for root's stable Git integration/R1/C6 schedule. Off-tree preparation is authorised.
- The explicit hook needs authentic compiler-provider closure; derived supported SpecTcl analysis remains Overdefined despite successful direct execution. Neither issue is waived.

## Parked unless needed by SpecTcl

- Remaining `string is/map/range/replace/insert/format` native compilers. Textual use by a hook is not evidence that a native compiler is required: its runtime handler may execute it correctly. No such native-compiler dependency is established by this resumed role yet.
- CompilerTraversal frame re-measurement against the 24 KB budget after the last split (`compiled_command`, `boxed_command_tokens`), unless a supported LSP/SpecTcl path establishes the need.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Contracts (1)–(3): entry-less whole-module compiles = fresh interpreter (BytecodeObject + resolved input; Direct fallback when the catalogue lacks a descriptor); compiled_preflight entry-less provider rule; fresh-world expression emission; profile-less → plain_tcl's engine, manifest restamped. compile_service:: 21/49 → 35/53 (0 pass→fail at that point).
- tcl-engine-tclvm lib 35/51 → 35/51 (0 pass→fail); its `a_whitelisted_ensembles_subcommands_run_wherever_they_are_called` also needs `string map` (no compiler yet).
- tcl-spectcl install.rs: pack rows get derived native compilers (override keeps shipped; new = NoHook; admitted alias_of = target's) + 2 tests.
- `string cat` native compiler (registry + codegen + runtime/rust); registry filter 4/4 (base 3/4).
- `dict update`/`dict with` release_compiled.
- Measured before → after (same binary, toggle): codegen_stamps 3/28 → 5/28 (heavy matrix skipped), tcl-vm word_value 3/3 → 3/3, mathfunc 7/9 → 7/9, cmd_string 13/51 → 13/51, tcl-debugger lib 17/17 → 17/17; tcl-compiler lib filters (compile_service, codegen, compiled_preflight, runtime_entry, inlining, optimiser, interprocedural, compilation_unit) 1101 → 1116 ok, 17 gains; 2 pass→fail were the old "missing entry ⇒ provider required" contract tests in codegen/native_failure.rs, rewritten to the user decision (unknown entry keeps the obligation; entry-less fresh compile presents the 8.4 compiler's own rejection, equal to a captured fresh entry's).

## Coordinator note (2026-10-22T13:25Z)

`optimiser/const_subst.rs` is assigned to compiler-codegen (`user-decisions-20261022T1317Z.json`; the user asked the coordinator to choose). Confirm the hypothesis with the fold probe first, then apply the proposed fix: the world barrier applies only to a declared `world_effects` footprint. Add a regression test showing that an undeclared-effect pack hook folds only the call it replaces, and that every other call stays fail-closed.
