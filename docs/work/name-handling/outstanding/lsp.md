# Outstanding work: lsp agent

Area: tcl-lsp-core, tcl-lsp-db and tcl-lsp-server (generic providers, server, WASM/WASI hosts).

Last updated: 2026-10-22T10:55Z

Maintained by the lsp agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## In progress

- tcl-lsp-server lib full run on the current tree (fresh binary, host-install survey now off in the test harness): collect failures, separate Server108-cache-caused from pre-existing; verify `fresh_analysis_is_reused_only_for_an_unchanged_input_revision`, `scan_workspace_folders_surveys_a_pinned_discovered_installation`, and the two 180 s tests (`workspace_symbol_caps_its_answer`, `startup_index_and_factory_reindex_keep_checked_document_roles`).
- Re-run lsp-core integration targets after the latest edits: hover_completion (past-end-of-line `$` completion clamp), force_import_shadow_consumers (references/rename agreement test), references_residual, name_resolution.

## Outstanding (in priority order)

1. tcl-lsp-core lib (3,485 tests, module filters) and tcl-lsp-db lib re-measurement on the current tree; tcl-lsp-server e2e.
2. Remaining clippy in my crates (`cargo clippy -p tcl-lsp-core -p tcl-lsp-db -p tcl-lsp-server --all-targets`); baseline 152 from Luna-41.
3. force_import not-shadowed halves (never passed on work): inline-proc / code-action offer no local `{args}` body; prepare_rename None (rename refusal is compiler §blocked).
4. Hand-back notes `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/lsp-NOTES.md` (draft exists; final before/after tables).
5. Review findings pending triage routing (do not start until routed): N03 (variable cursor scanner bypasses the dialect lexer; `profile_for_dialect("")` in rename), N04 (imported-command rename declaration-only edits), N05 (rename edits a forced-import call references excludes — my oracle-only ABSTAIN path already makes rename and references use the same `program_forced_import_abstains` decision; the in-document N05 shape is not covered), N06 (rename collision checks by target kind/origin), N07 (speculative proc-variable navigation used as a binding for rename), N11–N16 (package availability, incremental settlement vs fresh index, source views missing inherited namespace path, callback arity/semantic-token tails, minifier compaction, provider fallbacks).

## Blocked (on whom, and what is needed)

- perf agent / compiler: Analyser cost is quadratic in command count (n=10 3.1 s, 80 66 s debug; 41 empty procs 8.5 s; tk8.6 tclIndex tens of minutes) and `tcl_spectcl::bundled::load_embedded` 65–84 s. Blocks: `workspace_symbol_caps_its_answer` and `startup_index_and_factory_reindex_keep_checked_document_roles` (180 s), all WASI row-17 host checks, folder-add test time.
- compiler-names: object created by a relative name inside `namespace eval` is published at root (`::a`'s `rex` → `::rex`, `::b`'s → none) → name_resolution `tn_object_command_dispatch_is_scoped_to_its_creation_namespace`.
- compiler-names: rename refuses `unknown-completed-command-world` for `proc greet {} {…}; puts [greet]` (R1 not effective on this tree) — ~15 references_rename tests and force_import unshadowed rename; `incomplete-original-command-coverage` for class rename across two `namespace eval ::a` blocks; class `variable n` rename "original command unavailable".
- compiler: itcl classes declared inside `namespace eval ::app` do not join contract D, and class-proc calls inside class-member bodies → 4 itcl name_resolution tests + 2 references_rename.
- compiler (iRules realm): top-level `proc hit` in f5-irules — invocation lacks resolved reference / original lookup, so declaration selection breaks → `irules_switch_glob_body_is_followed`.
- compiler: registry source schema withdrawn when an operand reads an undefined variable (`regexp {plainword} $line`, `lsearch -exact $list $needle`) → 3 hover_residual tests.
- compiler-names: C6 (`namespace import ::a::Factory; Factory make`), C9 (W210 in pkgIndex `[file join $dir $missing]`).
- decision needed: references_residual `document_highlights_dedup_keeps_write_over_read_on_collision` (synthetic default AnalysisResult + injected reporting map, refused by the original-context contract) and `references_external_obj_method_inside_proc_body_subst` (tclsh contradicts the fixture: `$d` unset inside `proc run`).

## Done since the last update (with evidence: test counts, commit or snapshot)

- name_resolution 68→69/77; hover_completion 50→55/56; hover_residual 27→33/36; references_residual 16→21/23; references_rename 70→71/98; force_import 2→6/10; call_hierarchy 7/7; inlay_hints 9/9 (runs p4/p5, scratch logs).
- force_import ABSTAIN (decision 2026-10-22T07:18Z): `definition::program_forced_import_abstains`, consulted by definition/hover/references/signature/inlay/call-hierarchy prepare/inline-proc/rename/prepare-rename; 8 tests updated citing the decision; rename refuses and references omit any oracle-only-shadowed call (N05 consistency) + new agreement test.
- Server108: `tcl-lsp-server/src/fresh_analysis_cache.rs` (bounded LRU keyed on uri, text, dialect, admission, inputs epoch, factory generation, configured analyser inputs; cleared on sidecar change) + unit test.
- Host-install decision 2026-10-19T11:00Z: test harness presets an empty discovered-installation survey; pinned fixture-install test; per-thread plain-data memo of tclIndex surveys (`original_index_entries`) + unit test. `cargo check -p tcl-lsp-core -p tcl-lsp-server -p tcl-compiler --all-targets` clean.
- Contract D itcl `Class::proc` command_query branch (fn_to_tp_itcl_class_proc_colon_dispatch passes); class superclass/mixin operand references; declaration-name end-of-word cursor tolerance; constructor/destructor keyword hover; builtin/scoped-operation hover labels; caller-registry opt-in for builtin completion and intrep hover lines; scoped-body operation completion; 8.x namespace→global variable completion; undeclared-variable highlights empty; forced-import sibling test offsets on the call head word (decision 2026-10-18T16:00Z).
- Diagnosed WASI row 17 (inline spawn_blocking on one thread + analysis/pack-load cost) and the folder-add "hang" (host tclIndex analysis), see lsp-NOTES.md §4.
