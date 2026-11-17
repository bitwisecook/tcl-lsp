# Hand-over: LSP features

Written 17 November 2026 by the outgoing LSP features session. Nothing is held outside the repository: no local commit, stash, side branch or patch. The history of what was fixed and why is in [current state](../current-state.md) under "LSP features area, 15 November 2026"; this file is what a successor needs to start.

## Scope

Owns:

- `rust/tcl-lsp-core`, the whole library suite, except semantic tokens (`src/semantic_tokens.rs`, `src/semantic_tokens/**` and the token tests), which belong to the coordinating (tokens, owner) session.
- In `rust/tcl-lsp-server`, the navigation family end to end: the e2e groups `tcloo_navigation`, `rename_safety`, `definition`, `references`, `rename`, `navigation`, `name_resolution`, `issue945`, `issue1296_metaclass_chain`, `issue923_*`, and the server code behind them (references, rename, definition, call hierarchy, code lens, workspace symbols).

Does not own (coordinator ruling of 16 November): the server's `diagnostics`, `irules`, `recovery`, `code_actions`, `spec_packs`, `completion`, `hover`, `editor_features` and the non-navigation `issue*` groups, and all of `rust/tcl-lsp-db`. Those are the `server-e2e-db` area. That area agreed not to edit references, rename, definition, call hierarchy or code lens without saying so; lens and reference answers inside `editor_features` tests are still diagnosed here.

Compiler facts are not fixed here. A failure caused by a missing compiler fact goes to the owning area with the smallest fixture (see Hand-offs).

## How to measure

One package selection for the whole area, so the target directory is not rebuilt between runs:

```bash
source scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0
SEL="-p tcl-lsp-core -p tcl-lsp-db -p tcl-lsp-server"
# core library without token tests: about 7 minutes to run, 3,400 tests
cargo nextest run $SEL --lib -E 'package(tcl-lsp-core) & not test(semantic_tokens)' \
  --no-fail-fast --status-level fail --final-status-level none --failure-output final 2>&1 | cut -c1-700 > core.log
# server e2e: about 12 minutes to run, 1,650 tests
cargo nextest run $SEL --test e2e --no-fail-fast --status-level all --final-status-level none \
  --failure-output final 2>&1 | cut -c1-400 > e2e.log
# edit loop: one binary, filtered
cargo nextest run $SEL --lib -E 'test(type_definition)' --no-fail-fast --failure-output final
```

A rebuild after a rebase that brings compiler changes takes 8 to 12 minutes on the 4-core host; a one-line edit in `tcl-lsp-core` about one minute. `--test e2e` builds a different feature set from `--lib` and rebuilds `tcl-compiler` the first time. Run one build at a time.

Compare by failing-test list, not by count: extract the failing names (`grep -aE '^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV)'`), sort with `LC_ALL=C sort -u`, and `comm` against the baseline list. Since the file split a test path has one more module segment, so match on the last path segment (`sed 's/.*:://'`), which is unique enough within a module for this suite. Report "fixed N, new 0". A new e2e failure is rerun three times alone with `--test-threads 1` before it is believed.

## Current numbers

All before the file split; test names in the lists are the pre-split paths.

- `tcl-lsp-core` library outside semantic tokens: 3,410 run, 521 failing, on `work` at `2795a976` plus upstream to about 09:00 UTC on 16 November. Two of those were fixed afterwards in `105e577c`, so the list holds 519.
- Server e2e, whole suite: 1,651 run, 320 failing, on `work` at `7c741ce1` (15 November, 22:30 UTC). Thirteen were fixed afterwards (see the list note), so the list holds 307. It has not been re-run in full since; the coordinator's own run on 16 November counted 319.
- `tcl-lsp-db` was last measured at 80 failing on 15 November and is no longer this area's.

## Baseline list

[`lists/lsp-features-failing-2026-11-16.txt`](../lists/lsp-features-failing-2026-11-16.txt): 826 lines. Lines starting `tcl-lsp-core ` are the library failures (519); lines starting `tcl-lsp-server::e2e ` are the e2e failures (307, every e2e group, including the groups now owned by `server-e2e-db`). Tests fixed after each run were removed by name, not by a re-run.

## Queue

In order. Paths are on the restructured tree.

1. **Re-measure.** Run both suites on the restructured tree and replace the baseline list. Several upstream fixes landed after the last runs (`return [call]` selection, part of parse recovery, callback references from the `tk-oo` area) and the counts above are stale.

2. **Re-run what waits on the binding area** as each fact lands (fixtures are under Hand-offs): the `oo::define` command world (expected to be the widest cause in e2e `tcloo_navigation`, 30 failing, and in core `hover`, `definition`, `references` tests that reopen a class), the wildcard-import command world (8 core `workspace_index::tests` import-conflict tests, and probably another dozen wildcard-import tests there), class relation candidates on the unversioned profile (core `implementation` 8, `type_hierarchy` 9).

3. **Core `workspace_index`** (62 failing, the largest group). After removing the import-order tests, the rest are separate causes not yet traced per test: `oo::define` retraction and arrival across files, package provider order (`a_conditional_provide_does_not_make_a_document_the_provider`, `one_provider_required_from_many_documents_orders_each_of_them`), computed source paths, export gates. Files: `rust/tcl-lsp-core/src/workspace_index/` (`command_settlement.rs` holds the cross-document settlement added here), `rust/tcl-lsp-core/src/namespace_import.rs`, `rust/tcl-lsp-core/src/source_graph.rs`. Start by probing which need a `Selected` registry word after a wildcard import; those wait on item 2.

4. **Member rename on a release profile.** e2e `rename_safety` (about 12) and `tcloo_navigation` renames answer "the typed override family, configurations or receiver side are not closed for this edit". The code behind the remaining refusals reads reporting class maps, so an edit must come through `rust/tcl-lsp-core/src/original_member_rename.rs`. The tests that expect a refusal naming its reason ("not tracked") need the hazard walk in `rust/tcl-lsp-core/src/rename_safety.rs` to complete, which it does not over bodies the analysis did not enter (see Open questions).

5. **Requests against a moving index.** Two handlers were fixed (workspace symbols capture again; rename plans again, `Backend::rename_attempt` in `rust/tcl-lsp-server/src/rename/tiers.rs`). A `null` answer that passes when the test runs alone is probably the same shape in another handler: look for a `return Ok(None)` after `workspace_source_plan_is_current` or an index revision check. Known load-dependent flips: `issue1331_crossfile_diagnostics::a_genuinely_unknown_command_still_draws_w123`, `issue954_followup::*apply*highlight*`, `name_resolution::apply_lambda::tp_apply_body_and_bare_param_highlight`, `issue923_idx80_mathfunc::a_plain_cross_file_proc_call_resolves_by_default`, `issue1088_namespace_symbols::tp_empty_literal_is_the_global_namespace_at_global_scope`.

6. **Cross-file rename collision coverage.** e2e `rename::rename_from_shadowed_duplicate_proc_decl_reaches_cross_document_caller` and others answer "a completed original command world is unavailable for collision coverage": a consumer document whose only command is an unknown call (`List2array x`) has no completed command world of its own (`AnalysisResult::original_completed_command_world()` is `None`). The server passes conditional worlds for such documents; trace `collision_world` in `rust/tcl-lsp-core/src/original_command_rename.rs` to see why none is supplied.

7. **Lens counts for object dispatch.** e2e `editor_features::test_method_lens_counts_external_obj_dispatch_issue_864`, `test_property_lens_counts_my_dispatch_and_resolves_clickable`, `test_property_method_constructor_and_class_all_get_lenses_issue_992`; core `code_lens` 7. Counts differ where a member is reached through `my` in a body the analysis did not enter or through a typed receiver.

8. **Collapse duplicated tests** as modules are touched: the name-resolution matrix exists twice (`rust/tcl-lsp-core/tests/it_3/name_resolution.rs` and `rust/tcl-lsp-server/tests/e2e/name_resolution.rs`); keep one per layer and note before and after counts. Not started.

Not for this area until an owner decision (see Open questions): the minifier naming passes (core `minify`, 43) and anything needing a descriptor for a command in an unentered body (core `refactor::if_to_switch` 11, `switch_to_dict` 10, `inline_proc` 6, `call_hierarchy` 15).

## Held work

None. Every change is on `work`; there are no patches to apply.

## Open questions and rulings

Unanswered, for the owner:

- **Reading a command's descriptor in a body the analysis did not enter, for proof consumers.** The coordinator asked this area twice to implement it; it was declined both times because it widens what rename-grade code accepts and was not owner-reviewed. The coordinator accepted that and placed it with the analyser area. It blocks the refactors and call hierarchy listed above and the member-rename reasons.
- **Minifier naming passes.** `rust/tcl-lsp-core/src/minify/original.rs` carries `LEXICAL_NAMING_PASSES = false` from review N15: local renaming, alias insertion and expression rewrites are refused until a shared transactional rename planner exists. About 40 tests still expect those rewrites. Either the planner is built or the expectations are retired.

Rulings this area works under:

- Display consumers (hover, completion, document symbols, inlay hints, go-to-type-definition) may answer from written words and declared classes when the strict proof has none; rename, edits and refactors stay strict; a display fallback never feeds a proof consumer. The helpers are `declared_class_at` and `declared_self_dispatch_at` (`rust/tcl-lsp-core/src/definition/object_dispatch.rs`) and `declared_member_hover` (`rust/tcl-lsp-core/src/hover/class_members.rs`); `declared_class_at` excludes a class whose member set is incomplete (Clay).
- Find All References is a listing: it follows links across documents on a release profile (`linked_invocations_of`), rename does not (`invocations_of`).
- A call written before its procedure exists is no reference (tclsh 8.4 to 9.1 stop at `invalid command name`); the lens and Find All References both count zero. A declaration in the document keeps its own analysis as the answer for that document's calls; the order-free workspace index only supplies other documents.
- Procedure extraction on a release profile uses the shared plan after `reject_observable_motion`; `if_to_switch` keeps its obligation only where a variable trace cannot be ruled out. Contracts and evidence: `docs/design/analysis/name-resolution-proofs/refactor-original-extraction-motion-boundary.md`, `evidence/tcl/lsp-extract-proc-*.md`, `evidence/tcl/lsp-if-to-switch-*.md`.
- The protocol's rule for a column past the line end is applied at the server for signature help only; the core keeps its exact-cursor contract (`CurrentSourcePosition::capture` refuses a clamped position).
- Callback-selector reference spans in `method_symbol::readonly_local_reference_report` and `Instance { class }` on `BindingKind` belong to the `tk-oo` area, with the coordinator's agreement; about 25 core `references` callback tests wait on that, not on this area.
- From the file split: no Rust file over 2,000 lines, new code in a named module, new tests in the module's test file, a new module imports from the defining module.

## Hand-offs

Sent, waiting. Each was sent with these fixtures; all under `Analyser::new().analyse(src, "tcl8.6")` unless stated.

To the analyser and binding area:

- `oo::define`: after `oo::class create Widget {\n    method show {} { return shown }\n}\noo::define Widget method reopened {val} { return $val }\nWidget create w\nw show\n`, every later invocation has `resolved_command_reference == None` (`Widget`, `w`, and `return` inside the earlier bodies; `set` too in the `set o [Widget new]` form). Without the `oo::define` line all are `Some`. tclsh 8.6, 9.0, 9.1 print `shown hi again 1 set` for the equivalent script.
- Wildcard import: after `namespace eval ::dst {namespace import ::A::*}` with `::A` declared elsewhere, `source_structure::source_registry_words` answers `Conditional` or `SourceTransitions` instead of `Selected` for every later command in any namespace. tclsh 8.4 to 9.1: an imported `set` shadows only inside `::dst`; `::` and `::other` are unaffected; an unforced import at `::` fails with `already exists`.
- Class relations, profile `tcl`: `oo::class create Base {method run {} {}}\noo::class create Child {superclass Base; method run {} {}}\n` gives `logical_source_class_relation_candidates(Child) == None` (`Base` is `Some([])`).
- Linked reads: `proc wrap {} {\n upvar 1 other local\n return $local\n}` and `proc p {} {\n if {[info exists x]} { global x }\n puts $x\n}` have a variable root at the read but no occurrence and no alias template; core `declaration::tests` (3).
- Completed world: `proc greet {} { return hi }\ninterp alias {} sayHi {} greet\nputs [sayHi]\n` has no completed command world; with `sayHi` as a statement it has one. Six e2e renames.
- Earlier (15 November): the `foreach` loop-variable declaration occurrence; bare `test` after `namespace import ::tcltest::*`; registry words for commands in unentered bodies; callers in method bodies; object typing for widget paths.

To the diagnostics area: parse recovery. Fixed in `7a1d1983` for an unterminated bracket; still open for `namespace eval n {\nproc recovered_in_ns {} {}\n` (no diagnostic, nothing declared under `tcl8.6`) and for arity errors on tail commands. They asked which of the 18 e2e `recovery::*` tests still fail; that group is now `server-e2e-db`'s to report.

To the `tk-oo` area: W123 on a command an `auto_path` `tclIndex` provides, since `01568d90` (e2e `diagnostics::autoload_library_command_not_unknown_issue_832`). Acknowledged; they will fix it when resumed.

Received and not started: none. The `server-e2e-db` area listed navigation answers inside its groups for this area to diagnose: `issue1019_proc_args::tp_references_reach_a_bare_list_built_trace_callback`, `issue1281_ensemble_rename::a_dynamic_map_keeps_its_dispatch_word_unattributed_end_to_end`, `issue1302_import_builtin_shadow::*` (3), `issue1305_renamed_metaclass::*` (2), `diagnostics::autoload_library_command_references_and_rename_m8`, `diagnostics::consumer_rename_resolves_through_a_real_workspace_folder_m8`. Only the lens ones were looked at.

## Hazards

- `git rebase origin/work` on a shallow clone replays other sessions' commits when the merge base is outside the fetched depth, and stops on their conflicts. Run the connectivity check from the session instructions first, or deepen (`git fetch --depth=400 origin work`); with nothing local, `git merge --ff-only origin/work` is enough.
- `cargo fmt --all` (without `--check`) reformats files in other areas when another session pushed unformatted code. Format only your packages (`cargo fmt -p tcl-lsp-core -p tcl-lsp-server`) and check `git status` before committing.
- Chain the gates so a failure stops the push; a `;` between them once let a clippy failure through.
- Clippy runs with `-D warnings` over workspace dependencies too, so another area's lint failure in `tcl-compiler` fails your run. Fetch and rebase before concluding it is yours.
- A test that pins a compiler gap as its premise (`assert!(walk.structure(..).is_none())`) breaks when the compiler improves. Remove the premise, keep the assertion the test is named for.
- Core unit tests and e2e tests have contradicted each other (forward-call references). Before adding a display fallback that makes an e2e test pass, search the core tests for the same fixture shape with an interpreter note.
- Renaming a function a contract names breaks `cargo xtask owner-resolution`; keep the contracted name or update the row.
- Waiting for the workspace scan inside request handlers made the start-up race worse (1 of 15 passing); the e2e harness waits for start-up instead (`rust/tcl-lsp-server/tests/e2e/common/mod.rs`).
- Never edit the tree while `git bisect` has an old commit checked out, and when editing `current-state.md` stay inside your own heading: other areas' sections are adjacent.
- Disk: three incremental caches fill it. Keep incremental for the one selection above, prefix clippy and one-off selections with `CARGO_PROFILE_TEST_INCREMENTAL=false`, and delete `target/debug/incremental` when it fills. If rustc is killed with signal 9, set `CARGO_BUILD_JOBS=2`.
- A foreground command is cut off at 10 minutes; a rebuild after a rebase takes longer. Start it in the background or expect to wait on its output file.

## Oracle habits

- Interpreters in this container: `/root/.local/bin/tclsh8.4`, `tclsh8.5`, `tclsh8.6`, `tclsh9.0`, `tclsh9.1`. No `jimsh` and no Tcllib (the start-up hook failed to download it), so Jim expectations and the Tcllib-dependent tests could not be checked here.
- Write the fixture to a file in the scratchpad and run it under every release in one loop: `for v in 8.4 8.5 8.6 9.0 9.1; do echo "$v: $(/root/.local/bin/tclsh$v f.tcl 2>&1 | tr '\n' ' ')"; done`. Use `catch` and print a list so one run shows several cases.
- Make the fixture do what it claims before reading the result: a shadowing proc named `namespace` inside `namespace eval` changed what `namespace export` meant in one check and gave a wrong answer until `::namespace` was written out.
- Record the script and the per-release result in the test comment or the commit message; for an edit a proof consumer makes, write an evidence file under `evidence/tcl/` in the format of `docs/design/contracts/tcl-execution-evidence.md` and run `python3 scripts/dev/check-tcl-evidence.py`.
- To see what the analysis holds for a fixture, a temporary `#[test]` in the crate that prints with `eprintln!` and ends in `panic!()` is the quickest probe; remove it before committing.
