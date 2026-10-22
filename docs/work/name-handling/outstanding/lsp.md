# Outstanding work: lsp agent

Area: tcl-lsp-core, tcl-lsp-db and tcl-lsp-server (generic providers, server, WASM/WASI hosts).

Last updated: 2026-10-22T14:24Z

Maintained by the lsp agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

State: ACTIVE at 2026-10-22T14:21Z in `/workspace/naming-migration-active/tcl-lsp` (starting snapshot `d7224c706`). Native Default minify expression whitespace is first, per the 13:17 user decision; no new validation yet. Compiler-names has the first globally locked build window. Existing results below are agent-reported historical measurements, not validation/admission.

Current ownership: generic Core/DB/Server, including N10 class-command path selection, N11 package views and N13 inherited source views/convergence. Narrow shared Compiler ownership: `command_binding/original_literal_argv_rewrite.rs` and only the expression-gap adapter in `registry_invocation/source_structure.rs`, coordinated with compiler/compiler-names. `tests/e2e/spec_packs.rs` belongs to vm-runtime; BIG-IP modules and perf changes remain with their separate roles. Native minify retains Selected source/observer guards and validates exact configured expression tokens independently from the existing pure-value bracing proof. No cargo/build/test run yet in this environment.

Partial results of integration batch `p8` (current tree, built 13:01Z; evidence `merge-inventory-20261019/lsp/status-p8-*.txt`, `p8-failure-messages.txt`, `p8-summary.txt`):
- hover_completion **56/56**; hover_residual 33/36 (3 compiler §3c, unchanged); references_residual **23/24** (new negative test passes; ruling-B positive `references_external_obj_method_inside_proc_body_subst` still fails: `[$d bark]` at line 6 not reported even with `global d` — next: check whether the `global d` link reaches the method-dispatch reference path in `references.rs`/`method_symbol.rs`); name_resolution **70/77** (itcl ×4, C6, tn_object, C9); force_import_shadow_consumers **7/11** (4 never-passed unshadowed halves).
- stub_arg_roles 15/35 and lsp_edit_workspace 31/37: first measurement in this session, failures not yet classified. lsp_edit_workspace: 2 minify **compact** tests are N15 (now refused → change expectation per N15 rule, cite review N15 + decision 2026-10-22T11:00Z); 2 Default-tier minify tests (`minify_preserves_string_and_expr_results`, `minify_strips_comments_and_collapses_whitespace_factorial`) expect expression-internal whitespace removal (`{3+4*2}`) that the native "whole original words" path no longer does — NOT caused by N15; needs an origin/rust check and a decision; `linked_editing_links_namespace_relative_self_call`, `workspace_symbols_classmethod_surfaces_as_method` unclassified.
- references_rename: stopped after 14 tests (both N05 regressions pass; 3 failures so far), `status-p8-references_rename-partial.txt`. minify_residual: not run.

Current next checks (agent-reported work, not admission): after compiler-names → compiler-codegen → compiler → vm-runtime hand off the build lock, run Core `lsp_edit_workspace` Default minify selectors and the new `minify_residual::default_minify_compacts_expression_gaps_and_preserves_oracle_results`, then the full target and provider regressions. Core minify module, core/db/server library targets and clippy remain outstanding.

Source `/tmp/tcl-lsp-naming-env.sh`; keep the complete build/run/cleanup command under `flock /tmp/tcl-lsp-naming-build.lock`. Check free disk at least 2 GB (prefer 2.3) before every build. Logs drain after a 20 MB cap. Do not use old runners' unconditional `rm`: no executable younger than 60 minutes may be deleted, and no target cleanup may run while `pgrep -x cargo` or `pgrep -x rustc` finds a build. Fresh artifacts remain retained until eligible. No build/test has yet run in this environment.

Minify source edit is written and rustfmt completed (Compiler syntax-gap issuer + Core integration and six-profile regression), uncompiled. Manual original/minified-control fixture executions pass 12/12 across all five pinned C Tcl and pinned Jim; `/tmp/lsp-minify-manual-oracle-20261022.json`. This proves only that manual control's semantics, not the production minifier.

Files touched by me (all already in snapshots up to `ddb21ae4db` except docs): tcl-lsp-core `definition.rs`, `hover.rs`, `hover/original_registry.rs`, `completion.rs`, `completion/original_variables.rs`, `references.rs`, `rename.rs`, `method_symbol.rs`, `original_declaration.rs`, `call_hierarchy.rs`, `inlay_hints.rs`, `signature_help.rs`, `refactor/inline_proc.rs`, `package_resolver/original_names.rs`, `minify/original.rs`; tests `force_import_shadow_consumers.rs`, `hover_residual.rs`, `references_residual.rs`, `references_rename.rs`, `minify_residual.rs`; tcl-lsp-server `fresh_analysis_cache.rs` (new), `lib.rs`. Docs: this file, `merge-inventory-20261019/lsp-NOTES.md` (sections 1–5 written; §1/§2 numbers need the p8/p9 update), evidence dir `merge-inventory-20261019/lsp/`.

## In progress

- Native minify Default-tier expression whitespace fix; then hover schema withdrawal, N10 and remaining integration/provider/server proofs. Waiting for compiler-names first build window before any cargo invocation.

## Outstanding (in priority order)

1. tcl-lsp-core lib (3,485 tests, module filters) and tcl-lsp-db lib re-measurement on the current tree; tcl-lsp-server e2e.
2. Remaining clippy in my crates (`cargo clippy -p tcl-lsp-core -p tcl-lsp-db -p tcl-lsp-server --all-targets`); baseline 152 from Luna-41.
3. force_import not-shadowed halves (never passed on work): inline-proc / code-action offer no local `{args}` body; prepare_rename None (rename refusal is compiler §blocked).
4. Hand-back notes `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/lsp-NOTES.md` (draft exists; final before/after tables).
5. Review findings pending triage routing (do not start until routed): N03 (variable cursor scanner bypasses the dialect lexer; `profile_for_dialect("")` in rename), N04 (imported-command rename declaration-only edits), N05 (rename edits a forced-import call references excludes — my oracle-only ABSTAIN path already makes rename and references use the same `program_forced_import_abstains` decision; the in-document N05 shape is not covered), N06 (rename collision checks by target kind/origin), N07 (speculative proc-variable navigation used as a binding for rename), N11–N16 (package availability, incremental settlement vs fresh index, source views missing inherited namespace path, callback arity/semantic-token tails, minifier compaction, provider fallbacks).

## Blocked (on whom, and what is needed)

- (resolved by vm-runtime, pending my confirmation run) vm-runtime: tcl-lsp-server lib process aborts after ~221 tests — a tokio worker's thread-exit drops a thread-local PackHookHost → tcl_vm Value drop → `value::native_retirement::release` touches an already-destroyed TLS ("cannot access a Thread Local Storage value during or after destruction") → panic in destructor → SIGABRT. Needs `try_with` in the retirement path. Workaround: per-test processes.

- perf agent / compiler: Analyser cost is quadratic in command count (n=10 3.1 s, 80 66 s debug; 41 empty procs 8.5 s; tk8.6 tclIndex tens of minutes) and `tcl_spectcl::bundled::load_embedded` 65–84 s. Blocks: `workspace_symbol_caps_its_answer` and `startup_index_and_factory_reindex_keep_checked_document_roles` (180 s), all WASI row-17 host checks, folder-add test time.
- compiler-names: object created by a relative name inside `namespace eval` is published at root (`::a`'s `rex` → `::rex`, `::b`'s → none) → name_resolution `tn_object_command_dispatch_is_scoped_to_its_creation_namespace`.
- compiler-names: rename refuses `unknown-completed-command-world` for `proc greet {} {…}; puts [greet]` (R1 not effective on this tree) — ~15 references_rename tests and force_import unshadowed rename; `incomplete-original-command-coverage` for class rename across two `namespace eval ::a` blocks; class `variable n` rename "original command unavailable".
- compiler: itcl classes declared inside `namespace eval ::app` do not join contract D, and class-proc calls inside class-member bodies → 4 itcl name_resolution tests + 2 references_rename.
- compiler (iRules realm): top-level `proc hit` in f5-irules — invocation lacks resolved reference / original lookup, so declaration selection breaks → `irules_switch_glob_body_is_followed`.
- compiler: registry source schema withdrawn when an operand reads an undefined variable (`regexp {plainword} $line`, `lsearch -exact $list $needle`) → 3 hover_residual tests; compiler e1 probe now shows source schema survives, so ownership is routed back to LSP hover/registry selection for diagnosis.
- compiler-names: C6 (`namespace import ::a::Factory; Factory make`), C9 (W210 in pkgIndex `[file join $dir $missing]`).

## Done since the last update (with evidence: test counts, commit or snapshot)

- p8 (13:01Z tree): hover_completion 56/56, references_residual 23/24, name_resolution 70/77, force_import 7/11, hover_residual 33/36 (`lsp/status-p8-*.txt`). Saved srv4 per-test failure table `lsp/server-lib-srv4-failures.txt`. lsp-NOTES.md §2, §3, §5 written.
- Review N05 (pending measurement): `rename_in_program` refuses any edit that find-references from the same cursor does not report (overlap check, `unreferenced-edit`); `references_rename` N05 regressions (forced import, before/after `-force`, `forget`, redefinition after `-force`) assert edits ⊆ references and unchanged tclsh output for accepted renames. Cross-file case covered by force_import_shadow_consumers agreement test.
- Review N15 (pending measurement): lexical ("tcl") Compact/Aggressive minify now refuse alpha renaming exactly like the native path (`LEXICAL_NAMING_PASSES = false` in minify/original.rs; the old `compact_names` passes are no longer reachable from any public entry, including the server's minifyDocument); `minify_residual` N15 regression asserts the refusal and executed output on tcl / tcl8.6 / tcl9.0.
- references_residual rulings (A) real `incr x` write/read collision, (B) `global d` fixture + negative no-global case — applied, compile-checked, pending run.

- name_resolution 68→69/77; hover_completion 50→55/56; hover_residual 27→33/36; references_residual 16→21/23; references_rename 70→71/98; force_import 2→6/10; call_hierarchy 7/7; inlay_hints 9/9 (runs p4/p5, scratch logs).
- force_import ABSTAIN (decision 2026-10-22T07:18Z): `definition::program_forced_import_abstains`, consulted by definition/hover/references/signature/inlay/call-hierarchy prepare/inline-proc/rename/prepare-rename; 8 tests updated citing the decision; rename refuses and references omit any oracle-only-shadowed call (N05 consistency) + new agreement test.
- Server108: `tcl-lsp-server/src/fresh_analysis_cache.rs` (bounded LRU keyed on uri, text, dialect, admission, inputs epoch, factory generation, configured analyser inputs; cleared on sidecar change) + unit test.
- Host-install decision 2026-10-19T11:00Z: test harness presets an empty discovered-installation survey; pinned fixture-install test; per-thread plain-data memo of tclIndex surveys (`original_index_entries`) + unit test. `cargo check -p tcl-lsp-core -p tcl-lsp-server -p tcl-compiler --all-targets` clean.
- Contract D itcl `Class::proc` command_query branch (fn_to_tp_itcl_class_proc_colon_dispatch passes); class superclass/mixin operand references; declaration-name end-of-word cursor tolerance; constructor/destructor keyword hover; builtin/scoped-operation hover labels; caller-registry opt-in for builtin completion and intrep hover lines; scoped-body operation completion; 8.x namespace→global variable completion; undeclared-variable highlights empty; forced-import sibling test offsets on the call head word (decision 2026-10-18T16:00Z).
- Diagnosed WASI row 17 (inline spawn_blocking on one thread + analysis/pack-load cost) and the folder-add "hang" (host tclIndex analysis), see lsp-NOTES.md §4.

## Coordinator note (2026-10-22T13:20Z)

The Default-tier minify question is decided (`user-decisions-20261022T1317Z.json`): minify removes all whitespace it can, including inside braced expressions. `minify_preserves_string_and_expr_results` and `minify_strips_comments_and_collapses_whitespace_factorial` are correct as written; fix the native minify path to strip expression whitespace. No origin/rust check is needed for that decision.
- hover_residual (from compiler, 13:30Z): for `regexp {plainword} $line` the compiler still returns the pattern schema; the hover is lost in tcl-lsp-core (`hover.rs` or `original_invocation.rs`). This is yours.
