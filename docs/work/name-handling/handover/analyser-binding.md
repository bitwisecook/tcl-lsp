# Hand-over: compiler analyser and binding (the walk)

Slug `analyser-binding`. Written 17 November 2026 by the outgoing session. Last code push from this area: `b358a517` (16 November). Nothing here was built or measured after the file split.

## Scope

Owns, in `rust/tcl-compiler`: the binding walk and the realm (`command_binding.rs` and `command_binding/**`, `realm`), `registry_invocation/**`, `unit_scope`, `signature_scan`, `var_resolve`, `variable_bindings`, `dynamic_names`, `object_types`, `regex_source`. It edits `rust/tcl-registry` when a command's declaration (effects, completion, transitions) is what stops the walk.

Does not own: `analyser/diagnostics/**`, `analyser::handlers`, `param_traits`, `analyser::types` and the diagnostic integration tests (diagnostics); class factories, clay, class definition scripts, method bodies, named-instance cells, lenient-`tcl` display facts, widget receivers (class-facts); `cfg_builder`, SCCP, optimiser, interprocedural summaries (dataflow and values); taint and iRules checks (taint). A fix that needs a line in another area's file goes in as its own small commit and the owner is told.

## How to measure

Whole `tcl-compiler` suite, from the repository root:

```bash
source scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0
cargo nextest run -p tcl-compiler --no-fail-fast --status-level fail \
  --final-status-level none --failure-output never > full.log 2>&1
grep -aE "^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV|ABORT) \[" full.log | sed -E 's/.*\) +//' | sort -u > fail.txt
comm -23 previous.txt fail.txt | wc -l   # fixed
comm -13 previous.txt fail.txt           # new: must be empty
```

About 12 to 15 minutes to build after any source change and 9 to 11 minutes to run (13,126 tests). No test was excluded. A registry edit rebuilds everything; also run `cargo nextest run -p tcl-registry` (2,444 tests, under a minute) and `make xtask-gates`. Since the file split, test paths gained a module segment, so compare lists by the last path segment.

Gates before a push: `cargo fmt -p tcl-compiler -p tcl-registry -- --check` (not `--all`: it reformats other areas' files), `cargo clippy -p tcl-compiler -p tcl-registry --all-targets -- -D warnings -A clippy::assert_is_empty`, `make xtask-gates`, then fetch, rebase, **`cargo check -p tcl-registry -p tcl-compiler --tests`**, push.

Fast loops used for diagnosis:

- `handover-analyser-binding-probe-tools.patch` adds `rust/tcl-compiler/examples/zz_probe.rs` (do not commit it; a stale example breaks `cargo test --no-run`). `cargo build -p tcl-compiler --example zz_probe` takes about two minutes; `target/debug/examples/zz_probe <dialect> <file>` prints classes and diagnostics, `ZZOPT=1` adds optimiser rewrites, `ZZTAINT=1` taint warnings, `ZZCFG=1` each procedure's CFG statements.
- The walk trace lines (`ORIGINAL_SELECTION`, `ORIGINAL_WORLD_FINISH`, `ORIGINAL_COMMAND_CAPTURE`, ...) exist only in test builds. Put the test below next to `assert_no_static_proc_completion` (now in `rust/tcl-compiler/src/analyser/handlers/tests/classes/static_provenance.rs`), build once, then run any source without rebuilding: `ZZSRC='<tcl>' ZZDIALECT=tcl8.6 TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE=1 TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER=1 cargo nextest run -p tcl-compiler --lib --success-output final -E 'test(zz_probe_walk)'`. Remove it before committing.

```rust
#[test]
fn zz_probe_walk() {
    let owned = std::env::var("ZZSRC").unwrap_or_else(|_| "puts a\n".to_owned());
    for source in [owned.as_str()] {
        let mut analyser = Analyser::new();
        let dialect = std::env::var("ZZDIALECT").unwrap_or_else(|_| "tcl8.6".to_owned());
        analyser.analyse_and_retain_result_for_test(source, &dialect);
        analyser.registry = Some(analyser.profile_registry());
        eprintln!("ZZ == {source:?}");
        let all = analyser.statements_in_span(Span::new(0, u32::try_from(source.len()).unwrap())).unwrap();
        for seg in all {
            let config = analyser.file_lexer_config();
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &Analyser::source_map(&analyser.source, &analyser.cached_line_index, analyser.cached_line_index_source_len),
                config,
                &seg,
            );
            analyser.head_identities.stamp_original_tokens(&mut tokens);
            eprintln!(
                "ZZ   {:>3} {:<10} proved={:?}",
                seg.span.start(),
                seg.name(),
                tokens.source_binding.as_ref().map(|b| b.proved_execution_target().map(|t| t.command.clone())),
            );
        }
    }
}

```

Reading a trace: `ORIGINAL_WORLD_FINISH ... operations=["withdraw"]` is the command world being given up at that site; `command_return ... routes=[Unknown]` is a command with no declared completion or an opaque handler; a site with `ALIAS_CAPTURE` and no `COMMAND_CAPTURE` returned early from `prepare_native_handler`.

## Current numbers

Last complete run: 16 November 2026, on the tree of `b358a517` before the file split: 13,126 run, 2,242 failing. The area started at 13,082 run, 2,630 failing. Every push from this area was measured "new 0" against the previous list, with two recorded exceptions (see Hazards and Hand-offs).

## Baseline list

`docs/work/name-handling/lists/analyser-binding-failing-2026-11-16.txt`: the 2,242 failing tests of that run, whole crate (not only this area's modules), pre-split paths.

## Queue

In the order last agreed with the coordinator.

1. **Apply and measure the startup-variables patch** (Held work 1). It is the taint session's fixture 1 and much wider: any file-level read of `argv`, `env`, `tcl_version`, `tcl_platform`, `auto_path` ended the walk.
2. **Values session's walk block**, what is left:
   - `incr n`, `lindex $n 0`, `llength $n`, `set y [incr ::hits]`, `global g; catch {incr g}` over a value of unknown contents: blocked on the stock-object-type ruling (Open questions 1).
   - `uplevel #0 [list puts hi]`: the issue-1797 session announced the admission (a script word that is one command substitution of literal words) in `command_binding/original_delegated_script.rs`. Check whether it landed before touching it.
   - `cfg_builder::global_write_info::uplevel_caller_frame_is_not_a_global_write` still fails although the binding query is now exact (`variable_write_projection_with_metadata_context` answers `literal_names: ["n"]` for `proc ::p {} { uplevel 1 { ::set n 1 } }`); the remainder is in the summary (values).
   - S100 shimmer: `set v 5; expr {$v + 1}; lindex $v 0` needs the variable access at the `lindex` to carry the representation the earlier `expr` operand read left (`SsaSourceView::read_word_representation_advice` answers nothing; `contents_representation_at` after an arithmetic operand read). Not started.
3. **Taint fixtures.** (a) startup variables: item 1. (b) `set line [gets stdin]\nfoo $line\neval $line`: the walk does prove the later `eval` (`::eval`), so the missing sink candidates are in `resolved_tokens_invocation_with_metadata_context` or the taint consumer, not in the walk; not traced further. (c) Tk widget instance commands (`entry .user` then `.user get`) are not in the command world: now with class-facts by the coordinator's split.
4. **Loop-body lookup state for an uncalled procedure** (diagnostics): under tcl9.0, `proc p {items body} {foreach item {a b} {uplevel 1 $body}}`: the nested `uplevel` is a reached dispatch whose `InvocationBodyAssistance` has `unknown_residual: true`; the same command in an `if {1}` or `catch` body is closed with roles `[(1, Body)]`. Decides 11 `analyser::param_traits` tests. Not started.
5. **Walk cluster leftovers**: multi-word `eval`; a statement after a proved runtime error (`incr x {$n}`) has no lookup state; other inline intrinsics with no declared completion (symptom: `routes=[Unknown]` at `command_return`; fix pattern: `completion: Some(CompletionDescriptor::exact(...))` in the registry, as done for `namespace current/qualifiers/tail/code/upvar` and `info level`).
6. **Unentered-command descriptor query** for the LSP refactors. The four conditions are in `current-state.md` under the LSP features area (single literal bareword head; not rebindable anywhere in the document, sourced files and computed evals included; site never reached; source-reshaping consumers only). One query next to `registry_invocation::source_structure::source_registry_words`, returning a descriptor type distinct from the proved one. Unblocks `if_to_switch` (10), extraction and `inline_proc`. Tell the LSP features area when it lands. Not started.
7. **Two iRules inventory gaps** (`signature_scan::vendor_name`): no command listed at `when HTTP_REQUEST { if {"[class match ignored equals /Common/inert_dg]"} { set inert 1 } }` (command substitution inside a quoted expression operand), and none inside the first fragment of `eval { pool /Common/x } y`. Not started.
8. **Wildcard import from an undeclared namespace** closes the world for every later command (8 `tcl-lsp-core` `workspace_index::tests`). Oracle from the LSP features area, tclsh 8.4 to 9.1: an unforced import can shadow lookups only from the importing namespace, and at `::` it cannot replace an existing command. The ledger has no per-namespace "unknown additions" state today (`source_command_world::capture` withdraws the whole world); this is a design item, not a patch.
9. `unit_scope::tests::agreeing_callers_still_seed_through_a_substitution` (None, expected `Some("n")`): assigned to this area by the coordinator as an accepted regression of the variable-link landing. Not started.
10. Two `// dialect-drift-ok: unreviewed` markers in this area: `rust/tcl-compiler/src/analyser/utils/stub_directives.rs` and `rust/tcl-compiler/src/command_binding/module_bindings/namespace_transitions.rs`. Thread the real grammar and remove the marker when next touched.

## Held work

All three are in `docs/work/name-handling/patches/`. None is applied to `work`.

1. `handover-analyser-binding-startup-variables.patch`. Rebased by hand onto the restructured tree at `da039ed1` (`git am` or `git apply` there; `git apply --check` passes). **Not compiled after the rebase and never measured on the whole crate.** `variable_bindings::fresh_namespace_cells` (now `rust/tcl-compiler/src/variable_bindings/namespace_cells.rs`) listed the startup variables under authored string keys (`Authored("::argv")`); the walk addresses a global by `VariableCellKey::Namespace { identity: Authored("::"), simple }`, so `present.contains` missed, the closed table called the cell absent, and the read was a proved error. The patch seeds both key forms and adds `a_startup_variable_read_at_file_level_reaches_what_follows` (tclsh 8.4.20 to 9.1.0 print `1 1 1 1 1 1` for the `info exists` list in its comment). On the pre-split tree the test probe showed `puts $tcl_version`, `$env(HOME)`, `$tcl_platform(os)`, `eval $argv` proved, `puts $nosuchvar` and `unset argv; puts $argv` still errors.
2. `handover-analyser-binding-stock-object-types.patch` (was `walk-stock-object-types.patch`). Written against the pre-split layout at about `8343a33b`; `git apply --check` passes on `da039ed1`. Not to be applied without the ruling in Open questions 1. Its own test failed in the measuring run and was not examined.
3. `handover-analyser-binding-probe-tools.patch`: the diagnostic example described under How to measure. A tool, never to be committed.

## Open questions and rulings

Unanswered:

1. **Stock object types.** A list or numeric command over a value of unknown contents records `ObjectCallbackEffects::Unknown` (`command_binding/object_callbacks.rs`, `record_object_callback_effects`), which calls `mark_opaque_binding_mutation` and withdraws the command world for the rest of the body. That is deliberate: such a value may carry a foreign object type whose update-string or length hook runs anything (`docs/design/analysis/name-resolution-proofs/list.length-custom-string-update-world.md`, measured with a custom native object factory). Candidate rule: a supplied document with no host entry runs in a fresh interpreter that holds only its own object types until a command the walk cannot name (`load`) has run. Measured on the whole crate at about `8343a33b`: 44 fixed, 20 new. One of the 20 is the patch's own test; the other nineteen state the opposite rule for a supplied source:
  - `tcl-compiler analyser::bounds_checks::tests::w241_reads_a_proven_bound_and_step`
  - `tcl-compiler analyser::diagnostics::fp::rbs::fp_rbs_19_reporter_foreach_accumulator_silent`
  - `tcl-compiler command_binding::declaration_flow::tests::conditional_root_naming_is_scoped_to_the_original_command`
  - `tcl-compiler command_binding::declaration_flow::tests::declared_loop_first_entry_does_not_supply_runtime_zero_trip_proof`
  - `tcl-compiler command_binding::literal_object_pool::tests::scalar_list_length_needs_input_class_on_every_c_release`
  - `tcl-compiler command_binding::object_callbacks::tests::sequence_provider_result_does_not_preserve_unknown_or_replaced_handler_effects`
  - `tcl-compiler command_binding::object_callbacks::tests::stock_conversion_effects_stay_open_on_unknown_zero_trip_inputs`
  - `tcl-compiler command_binding::object_callbacks::tests::unknown_list_inputs_withdraw_callback_worlds_on_every_engine`
  - `tcl-compiler command_binding::source_loop::tests::pooled_empty_iteration_keeps_unproved_root_effects_open`
  - `tcl-compiler command_binding::source_loop::tests::unknown_iteration_input_keeps_conversion_callback_effects_open`
  - `tcl-compiler command_binding::source_representation::tests::unknown_outer_iteration_objects_keep_a_callback_residual`
  - `tcl-compiler executable_ir::tests::unknown_foreach_conversion_preserves_generic_later_dispatch`
  - `tcl-compiler optimiser::propagation::tests::retained_substitution_native_object_callbacks_withdraw_builtin_erasure`
  - `tcl-compiler shimmer::use_site::tests::an_unknown_abstract_input_does_not_close_the_result_store`
  - `tcl-compiler::it_2 value_transfer_witnesses::a_try_body_is_thrown_to_from_between_its_stores`
  - `tcl-compiler::it_2 w210_empty_loop_source::declared_empty_loop_reads_keep_real_entry_and_unknown_writer_controls`
  - `tcl-compiler::it_3 lsp_shimmer_conditional_source::compiler_checks_preserve_conditional_shimmer_without_action_authority`
  - `tcl-compiler::it_3 lsp_shimmer_conditional_source::forwarded_copy_warns_before_later_scope_refusal`
  - `tcl-compiler::it_3 lsp_shimmer_conditional_source::pure_and_pooled_literals_do_not_supply_integer_representation`
   The 44 were mostly shimmer (12), analyser (6), optimiser (5), core_analyses (5), intervals (4), taint (4). If the answer is yes, the nineteen move to host-entry fixtures and the patch lands; if no, those walk shapes stay opaque by design and the values area should be told.
2. BIGIP-NAME-069 (device): whether `proc` in a tmsh cli script or iApp implementation accepts a non-braced body. Until measured, `procedure_definition_body_policy` answers `BracedLiteralEstablished` for `Family::F5Tcl`.
3. Jim: no `jimsh` was installed in the container, so the Jim proof of the `eval`/`uplevel` literal-script receipt (`original_delegated_script.rs`) is still owed.

Rulings this area worked under (coordinator, relaying the owner where stated):

- Proof consumers keep the strict receipt; a diagnostic may read a weaker view when the pinned interpreter confirms the defect.
- The proving walk does not enter commands recovered after an unterminated delimiter. The recovered tail is an editor tier owned by the LSP server and diagnostics areas; the walk only needs to expose the recovered segmentation and the names the reached prefix rebinds.
- An F5 dialect is for F5 only: no Tcl core stands in for one. F5 questions that need a device go in `bigip-name-checks.md`, not guessed.
- A supplied analysis input stays exactly as supplied (`AnalyserState::resolved_analysis_input` no longer adds a vendor policy).
- For `upvar_info::nested_relative_one_reaches_past_the_caller`: there is no binding fact proving a closed caller set for an uncalled procedure, so the open case stays opaque and a test wanting exactness names its closed-caller premise.
- No Rust file over 2,000 lines; new code goes in the module it belongs to; a new module imports names from their defining module.

## Hand-offs

Sent, waiting:

- Taint: `it_4 taint_depth::interproc_depth::sanitising_helper_result_via_var_clears_t101` fails since `return [cmd]` lowers as a return (`a4e9d10e`). It passed only while the return was opaque; `proc len {x} { set r [string length $x]; return $r }` already drew T101. The helper summary does not apply the `string length` result colour.
- Dataflow: the variable-link half of the old patch (accept `StateTransition::VariableCellAlias` in `command_world_closed_on_normal_return`); the dataflow area recorded it as landed (`26c22812`). The linked-name fix it needed is `f1eeddf4`.
- Values: the caller-set answer above; `global_write_info::uplevel_caller_frame_is_not_a_global_write` remainder.
- Tk-oo: `autoload_library_command_not_unknown_issue_832` (server e2e) and the `thread::id` execution tests come from `01568d90`, not from this area.

Received, not started:

- Class-facts: `oo::class create ::nope::X {...}` withdraws the world because publication needs the missing qualifier namespaces ensured (tclsh 8.6 to 9.1 create them for objects, `proc` raises); refused in `selected_class_publication` (`command_publication.rs`, `publication_key_at`) and `OriginalNamedManufacture::capture`. Also `next` in an entered method body draws W123: `diagnostic_command_presence` looks the head up from `::` without the receiver helper path (`::oo::Helpers`); the uncalled-body route has it (`original_receiver_command_lookup`). Class-facts offered to take both, and asked whether to draft relative-head resolution for uncalled receiver methods: nothing is in flight here, it can.
- Server e2e: `AnalysisResult::logical_variable_scope_advice` lacks names bound by registry roles (upvar alias, `try ... on error {msg opts}`, `dict with` keys from a constant literal) and offers the procedure's locals inside `uplevel 1 {...}`; five `completion::*` tests. Also `namespace eval n {\nproc recovered_in_ns {} {}\n` under tcl8.6 gives no symbol and no diagnostic at all.
- LSP features: `logical_source_class_relation_candidates` is `None` for any class whose body holds a relation statement (probably class-facts); after `oo::define C ...` every later lookup in the document is conditional (tclsh: only that class's method table changes); callers inside method bodies record no invocation; no variable occurrence for a read of a linked local.
- Tooling: `source_graph::registry_words_for_original_call` answers `throw` under a supplied Tcl 8.4 availability context (tcl-diagram `original_diagram_metadata_keeps_actual_availability_and_source_currency`); lenient profile with release availability loses `source_registry_words` for `::string len abc`; `original_variable_rename_edits` edits only the `$fixed` use for `interp alias {} assign {} set fixed; assign 1; list $fixed` (changes program meaning); `foreach {first second}` has no rename plan.
- Diagnostics: declared-absence W210 after a possibly empty loop (`command_binding/declaration_flow`); arity subject chosen as the registry word where the dispatch is a user procedure in the current namespace (`close` in `namespace eval ::ns`).

## Hazards

- **I broke the build on `work` for about 25 minutes** (`d37ab705`, fixed by `fcbe6f2d`): measured, then rebased over a commit that changed a field's type, then pushed without building. Always `cargo check -p tcl-registry -p tcl-compiler --tests` after the final rebase.
- Do not edit tracked sources while a whole-crate build is compiling; the run then measures a mixed tree.
- Compare a run only against a list from the same base. After a rebase the "fixed" count includes other areas' commits, and "new" can include tests they pinned as failing (`w210_first_iteration_read_in_a_loop_body_fires` arrived that way).
- A guess costs a 15-minute rebuild. Two guesses in this stretch were wrong (the `::set` projection, the startup-variable key); one line of instrumentation settled each. Instrument first.
- The same fault appeared three times: a table keyed by typed cell (`VariableCellKey::Namespace` or `Activation`) queried or seeded by spelled name, which selects `Authored` keys only (`VariableCellTable::get(&str)`, `VariableCellSet` built from strings). Suspect it whenever something present reads as absent.
- A change that closes the command world unmasks consumers that relied on it being open (the optimiser barrier for O127, the global-write summary, existence folds). Expect downstream failures from a correct walk fix and trace each before calling it a regression.
- `git checkout rust` is ambiguous with the branch `rust`; use `git checkout -- rust/`.
- A subcommand with no `side_effects` inherits its parent command's; `namespace` subcommands inherited a namespace write and withdrew the world.
- Disk is a fixed allowance: one `target` directory only.

## Oracle habits

Interpreters: `/root/.local/bin/tclsh8.4`, `tclsh8.5`, `tclsh8.6`, `tclsh9.0`, `tclsh9.1` (8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0). No `jimsh`. For each changed expectation: write the smallest script that prints the observable, run it on every release the test names (`for v in 8.4 8.6 9.0 9.1; do echo '<script>' | /root/.local/bin/tclsh$v; done`), and put the script and its output in the test's doc comment. Use `catch {...} m; puts $m` for error text and `tcl::unsupported::representation` / `tcl::unsupported::disassemble` (8.6 and later) for object types and compiled forms. An expectation is changed only with that evidence on the test; an assertion is never weakened and no test is ignored.
