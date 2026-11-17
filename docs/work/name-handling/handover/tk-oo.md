# Hand-over: tk-oo (Tk and TclOO issues)

Written 17 November 2026 by the outgoing session, paused since 16 November. `work` was at da039ed1 when this was written.

## Scope

The issues assigned to this area: #2324 (Tk widget receivers and computed heads), #2427 (bounded stock `unknown`, T100 lost across a caught call), #1956 (Clay classes navigate), #1655 (Clay factory-prefix abstentions), and #832 only as a regression this area caused.

Code touched:
- tcl-registry: `state_transition.rs` (`PackageTransition::IndexedAutoload`, `command_binding::CREATES_WIDGET`), `commands/tcl/unknown.rs`, `commands/tcl/socket_.rs`, `commands/tk/*` (37 constructors), `definer.rs` and `definer/clay.rs`.
- tcl-compiler: the bounded-autoload logic in `command_binding.rs` (`autoload_defines_nothing`, `absent_head_runs_nothing`, `absent_head_reaches_only_stock_autoload`, `host_extensible_command_table`); the taint guard in `classify_substitution_taint`; per-call receiver classes in `interprocedural.rs` and `interprocedural/eager.rs`; Clay definition parents in `analyser/provider_definition.rs`; the self-dispatch catalogue in `class_hierarchy/original_metadata/source_method_reference.rs` and `command_binding/source_transition_advice/body_effects.rs`.
- tcl-lsp-core: `references.rs` callback scan, `method_symbol.rs` `readonly_local_reference_report`.

Not this area: the lsp-core callback scan as a whole (LSP features owns it; this area only added the unentered-body spans under a ruling), taint fixtures (taint session), the VM thread test `workers_inherit_a_separate_host_command_surface` (execution session; it failed before this area's commits).

## How to measure

Build flags for every run: `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0`. Never `--release`.

- Compiler, the main suite: `cargo test -p tcl-compiler --lib --tests --no-fail-fast -- --skip depth_guard::tests::the_source_walk_cap_fits_its_stack_budget`. About 20 minutes cold, about 8 to 10 warm. The log is about 210 MB; extract the failing lines and delete the log.
- `cargo test -p tcl-registry --lib --tests --no-fail-fast`: green at the last run.
- `cargo test -p tcl-lsp-core --lib --no-fail-fast`: about 5 minutes of test time.
- `cargo test -p tcl-lsp-db --test compiler_check_corpus`.
- The two regressions: `cargo test -p tcl-codegen --test it_2 -- catch_in_while_condition` and `cargo nextest run -p tcl-lsp-server --test e2e -E 'test(autoload_library_command_not_unknown_issue_832)'`.
- VM thread tests: `cargo test -p tcl-vm --test it_4 -- cmd_thread_e2e`.

Baseline comparison: build the same suite on `origin/work` in a second worktree with its own `CARGO_TARGET_DIR`, and compare failing names. Since the file split, a test's path gained a module, so compare by last path segment. I keyed compiler lists as "binary name + test path" because the it_* binaries reuse names.

## Current numbers

- tcl-compiler, `--lib --tests`, on 704605409 (16 November 2026): 10,765 passed, 2,356 failed. `origin/work` at that time had 2,359; there were no new failures.
- tcl-lsp-core lib, on issues/tk-oo-autoload-wip as it stood on 15 November (before the last #1956 fixes): 3,105 passed, 540 failed. With the final #1956 commit, the `references` group showed no failure that was not already in the baseline.
- tcl-registry: green. tcl-lsp-db `compiler_check_corpus`: 2 failures, both also on the baseline (`compiler_check_memo_matches_uncached_for_irules_flow_checks`, `..._for_hidden_writes`).

## Baseline list

`docs/work/name-handling/lists/tk-oo-failing-2026-11-16.txt`, one test per line, with the crate and binary in front:
- the compiler run above (2,356 lines);
- the lsp-core run of 15 November (540 lines), which predates the final #1956 commit, so some `references::` rows there are now green.

## Queue

Work these in order.

1. **#832 regression.** `tcl-lsp-server` e2e `diagnostics::autoload_library_command_not_unknown_issue_832` has failed since 01568d90.
   - **Fixture:** `libraryPaths` points at a directory whose `rbc/tclIndex` sets `auto_index(Rbc_ActiveLegend)`. The document `Rbc_ActiveLegend .g\nRbc_ZoomStack .g\n` gets W123.
   - **Cause (not traced, hypothesis):** `command_binding.rs` `autoload_defines_nothing` only consults `trusted_source_modules[..].autoload` and the catalogue. The server's library-path package database supplies its `tclIndex` names some other way, so the bounded-autoload rule treats those names as undefined.
   - **Fix:** when the entry or workspace supplies an autoload index (any `tclIndex` name, or simply any library path), an undefined head must stay unbounded.
   - **Witness:** the e2e test. Tell the LSP features session when it is on `work`.

2. **`codegen::catch_in_while_condition`** (`rust/tcl-codegen/tests/it_2/codegen.rs`, around line 2048). The execution session reports that it failed after 704605409 and passes with it reverted.
   - **Fixture:** `proc retry_loop {} { while {[catch {some_op} result] != 0} { puts "retrying..." }; return $result }`, lowered with the test-only `lower_to_ir`. It expects a `<cond>` call in the CFG.
   - **Cause:** 704605409 marks `lower_to_ir` IR host-extensible. The undefined `some_op` is therefore an unbounded head again, and the condition's `<cond>` statement is no longer emitted.
   - **First check:** did it pass before 01568d90? `git log -S catch_in_while_condition` on that file.
   - **Decide which applies:**
     - If the test was written or changed after 01568d90 and relied on the bounded rule, give the fixture a defined command (`proc some_op {} {}`) with a `#2427` comment.
     - If the `<cond>` effect should hold for an unbounded head, that is a lowering gap in the condition emission.
   - Tell the execution session either way.

3. **#1956: land it.**
   - **Patch:** `handover-tk-oo-1956-callback-references.patch`. Re-apply it on the split `references.rs` modules, rerun `cargo test -p tcl-lsp-core --lib` against a baseline, and land it when nothing new fails.
   - **Close #1956 if green.** The commit message carries `Closes #1956`. The tcllib httpd `connect` witness (lines 1011 and 1014) and the cat `Post` fixture passed on the branch.
   - **Tell LSP features which tests it turns green.** On 15 November these were `references::tests` …:
     - `references_reach_a_list_built_self_after_callback`
     - `references_reach_namespace_wrapped_my_callback_even_when_private`
     - `references_reach_namespace_wrapped_self_callback_only_when_public`
     - `the_real_tcllib_callback_shapes_resolve_to_their_methods`
     - `references_from_decl_reach_a_list_built_self_callback`
     - `qualified_tcloo_self_helper_builds_the_same_callback`
     - `references_reach_a_list_built_self_command_prefix`
     - `references_reach_a_list_built_self_object_callback`
     - `references_for_method_includes_one_hop_stored_callback_prefixes`
     - `composed_callback_prefix_references_keep_public_and_private_original_selectors`
     - `references_include_bare_call_to_a_proc_installed_into_oo_helpers`

4. **#2324: finish it.** The publication half is in `handover-tk-oo-2324-widget-publication.patch`. Remaining, both approved by the coordinator under #2324 (do not split them off):
   - **(a) Instance binding kind.**
     - **Change:** add `CommandBindingDefinitionKind::Instance { class: &'static str }` in tcl-registry `state_transition.rs`, set by `CREATES_WIDGET` from `ObjectClassSpec::class_name`, and `BindingKind::Instance { class }` in tcl-compiler `command_binding.rs`. The walk's `Define` arm (`apply_may_binding_transition`) installs it.
     - **Effects:** a call through such a head takes the selected method's declared effects when the method word is literal, via `CommandRegistry::instance_method_selection_for_descriptor_in_context`. Otherwise it takes the union of the class's instance methods. An unknown method stays an unknown call.
     - **Handle every match:** every match on either enum gets an explicit arm, with no wildcard folding it into `Class`. Tk's own `destroy` subcommand is not a deletion.
     - **Commits:** the enum change and the consumer arms as separate commits.
     - **Who to tell:** the burn-down coordinator was told the consumer list, and notified the binding, dataflow, values, taint, diagnostics and tooling sessions.
   - **(b) A new `NativeResultSelection`** in tcl-registry `native_result.rs` for "a fresh string equal to argument N". Tk returns a new `Tk_PathName` object, so `Argument(N)`, which shares identity, is wrong. Do not use a `const_fold`: that would let substitution consumers offer to replace a constructor call. Pin with tests that the constructor is never folded away, on Tk 8.6 and 9.0.
   - **Witnesses:**
     - `set e .e; entry $e; puts [$e configure]` and `puts [[ttk::entry .e] configure]` give no W307, and the walk stays seen after the call.
     - An unknown widget method behaves as an unknown call.
     - The taint session's three fixtures report T100 on `value`, all tcl8.6 through `find_taint_warnings_for_cu`:
       - `package require Tk\nentry .user\nset value [.user get]\neval $value`
       - the same with `ttk::entry .user`
       - `set widget [ttk::entry .user]\nset value [$widget get]\neval $value`
     - Taint tests: it_3 `taint::object_instance_taint_sources` (35) and lib `taint::instance_classes` (5).
   - **Full suite:** zero new failures, then close #2324 and tell the taint session.

5. **#1655** (after 1 to 4).
   - **Problem:** the tcllib clay corpus test (lsp-db `compiler_check_corpus` clay inventory, and `class_factories`) records no classes.
   - **First step:** bisect it. A shallow clone needs `git fetch --deepen=3000 origin work`, and there is no git history before 2026-06-18.
   - **If it is a regression:** fix it.
   - **If it is a deliberate burn-down rule:** message the burn-down coordinator with the culprit commit and the proposed fix before changing it.
   - **Then:** lift the two abstention limits in clay's factory-prefix walk, and close #1655.
   - `analyser::provider_definition::tests::clay_inventory_refuses_computed_expanded_and_replaced_definers` already failed on the baseline.

## Held work

- **`docs/work/name-handling/patches/handover-tk-oo-1956-callback-references.patch`.**
  - **Applies to:** 704605409, the pre-restructure layout. It conflicts with the split `references.rs`. This is commit 3a5322f02, also on the remote branch `issues/tk-oo-1956-callbacks`.
  - **What it does:**
    - adds `references::unentered_callback_reference_spans`, with receiver-aware `[self]` visibility;
    - resolves a literal `my` head in an unentered `[list my m]` builder, abstaining when the document defines, renames or aliases `my`;
    - skips prefix wrappers (`namespace code`) as sinks;
    - joins the spans into `method_symbol::readonly_local_reference_report` only;
    - adds the test `an_unentered_callback_selector_is_a_reference_but_never_a_rename_edit`.
  - **Verified:** the lsp-core `references` group had no new failure against the baseline.
  - **Unverified:**
    - The full lsp-core lib run was stopped by the pause.
    - The rename-hazard helper `document_renames_command` is a textual over-approximation (`proc`, `rename`, `alias` followed by the name). It only abstains, so it is safe but crude.
- **`docs/work/name-handling/patches/handover-tk-oo-2324-widget-publication.patch`.**
  - **Applies to:** 704605409, the pre-restructure layout. It conflicts with the split `var_command.rs`. It is the registry and test part of the remote branch `issues/tk-oo-autoload-wip` (65e16b2a4).
  - **What it does:**
    - adds `command_binding::CREATES_WIDGET`, which defines the literal path as an Object command, `OnOkOnly`, with no dynamic widening. A computed path, or one not starting with `.`, states nothing. Basis: `Tcl_CreateObjCommand`, then `Tk_DestroyWindow` on a configure failure, in `tkEntry.c` for Tk 8.4.20 to 9.0.4.
    - attaches it to all 37 Tk constructor specs;
    - adds the test `analyser::diagnostics::var_command::tk_widget_path_tests::a_head_holding_a_created_widget_path_is_a_present_command` (tcl8.6 and 9.0 under the Tk provider fixture).
  - **Unverified:**
    - That test still failed at its first run on the WIP branch (its second assertion, W307 absent). Probe runs showed `.e` present but W307 still firing for `set e [entry .e]; $e configure`, because the variable is not typed. Part (b) above is what clears it.
    - The `set e .e; entry $e; puts [$e configure]` shape gave no W307 in the probe.
    - No full suite was run with this change.
    - The `tk_preview` uncertainty-cap test regressed under the first version, which widened on a computed path. The current version should not regress it, but that is unchecked.
- **Remote branches.** `issues/tk-oo-autoload-wip` also holds the #1956 WIP commits, superseded by the patch above. `issues/tk-oo-1956-callbacks` holds the squashed #1956 commit. Neither is rebased onto the split tree.

## Open questions and rulings

Rulings this area works under:
- **#2427 (coordinator).** Tests that used a bare undefined command as an "unknown mutation" device now name it in `::auto_index` with the comment `// #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.` Tests asserting real behaviour were fixed in the code, not the test.
- **A catalogued name (coordinator):** one the registry knows in any dialect (`known_in_any_dialect`) keeps the unbounded fallback and its catalogued taint semantics. This is why the iRules-under-tcl8.6 colour tests stay unchanged.
- **Host commands (burn-down coordinator).** An execution compile's command table is not the catalogue. On `work` (704605409) the entry fact `host_extensible_command_table` is set for the compile service's fresh document, standalone source rewrites (the optimiser) and the test-only `lower_to_ir`; editor analysis keeps the bound.
- **#1956 (burn-down coordinator):**
  - Callback selectors in unentered bodies are read-only reference spans. Rename stays strict: `unproved_callback_method_selector` remains the whole-rename hazard.
  - The scan must abstain when the document rebinds the dispatch word or when the selector is not a single literal.
  - Pinned by the test named above.
- **#2324 (coordinator).**
  - `BindingTarget`/`BindingKind::Instance` and the fresh-string result selection are both under #2324, with no `const_fold`.
  - Before editing the consumer files, send the burn-down coordinator the enum change and consumer list. That was done on 15 November, with no objection.
  - Rebase immediately before committing; the enum change and the consumer arms are separate commits.
  - Run `make xtask-gates` and the whole `tcl-compiler` suite against a baseline before landing.
- **File split (owner).** No Rust file over 2,000 lines (`cargo xtask file-sizes --check`). New tests go in the split test modules, never back inline. New code goes in a named module.

There are no unanswered questions from this area.

## Hand-offs

Waiting on others:
- The execution session owns `workers_inherit_a_separate_host_command_surface`. The taint session was told that `classify_substitution_taint` broke clippy's `too_many_lines` (103/100) after its 43c8d0b7.

Promised, not yet done:
- Tell the taint session when #2324 is on `work` (they will re-run their fixtures).
- Tell the LSP features session when the #832 fix and #1956 are on `work`.
- Tell the execution session the verdict on `catch_in_while_condition`.

## Hazards

- **Shallow clone.** The clone is shallow: `git rebase` of a remote branch with no merge base tries to replay thousands of commits. Cherry-pick the named commits instead. Bisecting needs `git fetch --deepen=3000`.
- **Disk.** The disk filled once (two target directories plus the 6 GB docs tree). A baseline worktree should be a sparse checkout without `docs/design` and `docs/work`, with its own `CARGO_TARGET_DIR`. Delete the 200 MB compiler-suite logs once the failing lists are extracted.
  - Docs tests fail in such a sparse worktree (`wasm_api_architecture::*`). They are not real "fixed" or "new" rows.
- **Shared target directory.** Sharing one between a bisect worktree and the main tree left a stale `tcl-cmd-core` artefact (a bogus E0599). `cargo clean -p tcl-core-types -p tcl-cmd-core` cured it.
- **Struct literals.** `SourceAnalysisEntry` is destructured exhaustively in `command_binding/source_analysis_cache.rs`. A new entry field must be added there and in the cache key, or the build breaks.
- **Clippy limits.** Clippy rejects more than three bools in `Lowerer`; use a two-variant enum. `too_many_lines` sits at 100: a new field initialiser can push a long function over.
- **Debug traces.** Never commit them. One commit with tracing was pushed to a WIP branch and then removed; only the squashed commits reached `work`.
- **`after` and `my`.** `after` runs a bare `[list my m]` in the global namespace, where `my` does not exist. Fixtures for unentered `my` callbacks must use `namespace code [list my m]`.
- **`lower_to_ir` has two consumers.** It is shared by execution tests and by compiler unit tests. Changing its entry facts moves both: 704605409 had to restore five compiler expectations, which came back to their pre-01568d90 form.

## Oracle habits

- **Tcl releases.** `tclsh8.4`, `tclsh8.5`, `tclsh8.6`, `tclsh9.0` and `tclsh9.1` are wrapper scripts in `/root/.local/bin`; plain `tclsh` is 9.0.4. Pipe a script in: `printf 'puts [info patchlevel]\n' | tclsh8.6`.
- **C sources.** These are in `tmp/tcl8.4.20` to `tmp/tcl9.1.0` and `tmp/tk8.4.20` to `tmp/tk9.1.0`. The stock `unknown` and `auto_load` are in `library/init.tcl`; Tk widget creation is in `generic/tkEntry.c` and `tkWindow.c` (`Tk_CreateWindowFromPath`); `socket` option scanning is in `generic/tclIOCmd.c` (`Tcl_SocketObjCmd`).
- **tcllib.** tcllib 2.0 is in `tmp/tcllib-2.0`; clay is `modules/clay/clay.tcl` (0.8.8) and httpd is `modules/httpd/httpd.tcl`.
- **Jim.** No Jim interpreter was used in this area.
