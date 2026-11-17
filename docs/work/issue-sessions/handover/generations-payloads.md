# Hand-over: procedure generations and diagnostic payloads (#1797, #2426)

Written 17 November 2026 by the outgoing session, paused since 16 November, as a message to the orchestrator and filed here unedited. The successor is the `generations-payloads` issue session.

The paths and commits below predate the file split of 16 November: see *Starting* in `../README.md` before applying anything.

```text
HANDOFF for #1797 and #2426. Branch issues/generations-payloads, pushed, clean tree. NOTHING of mine is on `work` yet.

1. ISSUES
- #2426 (open): both remaining items done in 37b9e06f. (a) Per-family payload contract for every compiler_checks::Diagnostic code, in docs/design/contracts/diagnostic-subjects.md § Compiler-check payloads, owned by CheckPayloadKind::accepts + CheckPayload::CODES (compiler_checks.rs). Test: compiler_checks::payload_contract_tests. (b) Serializer round-trips: tcl_lsp_core::check_payload (CheckPayloadData, v1, names as byte arrays, family_fixtures behind feature `transport-fixtures`) + one wire test per adapter (tcl-cli diag.rs, tcl-mcp tools.rs, tcl-lsp-server lib.rs: check_payload_wire_tests). Before close: fast-forward work, post closing comment, close completed. Caveat for the comment: O106, T106, W201, IRULE4002, IRULE4004 are only checked at the conversion level (every_contract_code_converts_to_its_family_payload); their producers do not fire on work (pre-existing: path_concat::tests::flags_manual_path_with_var_interp, irules_checks::tests::irule4002_fires_for_generic_static_name, irule4004_literal_set_in_per_request_event_fires all fail on base). IRULE3102 is not emitted by run_all_checks (analyser owns it), so it's not in CODES but keeps its payload variant.
- #1797 (open): implemented in 92bfa3f0 per decisions in issuecomment-6902375699. Before close: fast-forward work, closing comment, close completed.

2. COMMITS
- 37b9e06f Carry a typed payload on every compiler-check diagnostic (Refs #2426)
- 92bfa3f0 Publish recovered procedure bodies with precise summaries (Refs #1797)
Both on top of origin/work ef227ae4. work does not have them.

3. VERIFICATION
Env: export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0 CARGO_TARGET_DIR=<repo>/target
Build: cargo test -p tcl-compiler -p tcl-lsp-core -p tcl-cli -p tcl-lsp-server --lib --no-run ; cargo test -p tcl-mcp --bins --no-run (tcl-mcp is a bin crate).
Run the printed binaries directly from each crate dir (rust/<crate>), compiler with: --skip depth_guard::tests::the_source_walk_cap_fits_its_stack_budget --test-threads 2or3 -q. Failure list: grep -E "^    [a-zA-Z_:0-9]+$" log | sort -u; compare with comm -13 base new.
Baseline: git worktree add <dir> origin/work, build there with its own CARGO_TARGET_DIR, same commands. ~1700 compiler, ~535 lsp-core, ~136 server, 3 cli, 3 mcp failures pre-exist.
Results at base 5eec806b (prior rebase): compiler 0 new / 8 fixed (alias_target_prefix_resolves..., conditional_namespace_alias..., late_namespaced_alias..., distinct_body_summaries..., qualified_call_resolves_via_qualified_key, constructed_namespace_eval_defers..., constructed_procedure_roots..., constructed_tcloo_method...); server 0 new/2 fixed; lsp-core, cli, mcp 0 new.
At ef227ae4 (current): all new tests pass (compiler filter: a_recovered literal_list_scripts each_call_joins_only_its_own literal_word_script payload_contract_tests -> 10 ok; check_payload in core/cli/mcp/server ok). Clippy clean: cargo clippy -p tcl-compiler -p tcl-lsp-core -p tcl-cli -p tcl-mcp -p tcl-lsp-server -p tcl-lsp-db --all-targets -- -D warnings -A clippy::assert_is_empty (prefix CARGO_PROFILE_TEST_INCREMENTAL=false). Gates OK: registry-axes, value-transfers --check, owner-resolution, callback-inventory --check, diagnostic-presentation. rustfmt --edition 2024 applied.
STILL TO RERUN: full compiler + server suites at ef227ae4 (I killed it for the pause), against a fresh ef227ae4 (or newer) baseline. A rebase conflict in tcl-lsp-server/src/lib.rs (both sides appended a test module at EOF) was resolved by appending my module after origin's; check that file compiles if rebasing again.

4. DESIGN DECISIONS
#1797 (decisions: issuecomment-6902375699):
- Literal-list transparency: new command_binding/literal_word_script.rs. A script word that is exactly one [cmd w...] of Literal/BracedLiteral words, with an exact produced value (arguments.literal_at(i) Some), is admitted like a braced literal in original_delegated_script::literal_script and OriginalNamespaceBodyDelegation::capture. Without this, source_command_world::capture pushed Operation::Withdraw for any non-braced namespace-eval/eval body, which withdrew the world and killed later proc publication. $x stays fail-closed.
- Decision 2 (proc proof at recovered script's point) is realized by the source walk's own resolution: witness a_recovered_script_publishes_no_body_through_a_shadowed_proc.
- Recovered bodies were already lowered as Installed / OriginalDeclaration body units with ProcedureIds (lowering/installed_bodies.rs from SourceCommandBindings). New Module::recovered_procedure_bodies / summary_procedure_bodies / recovered_procedure_for_target (exact implementation allocation) in ir.rs; RetainedSourceModuleBindings::matches_summary_body; frame_effects, global_write_info, cfg_builder/mod.rs iterate summary_procedure_bodies.
- summary_candidates::original_head falls back to binding.original_written_name_input(tokens,0) because value-channel scripts aren't SourceChannel::Document.
- ModuleCommandBindings::entered_body_namespace: namespaces join by intersection, so the observed state loses ::n; summaries now resolve with the body's authored namespace (+ancestors) in `namespaces` and its C geometry in original_command_world (SourceOriginalCommandWorld::enter_authored_namespace). This also fixed every namespaced proc's summary (was opaque).
- Owner contract row updated in shared-utility-contracts-rust.md; docs in docs/design/compiler/original-procedure-analysis.md.
- I messaged naming parent session_017u5g3KpKgjfMZcPMj8rx4F before editing original_delegated_script.rs / original_namespace_body.rs. No reply was received.
#2426: Diagnostic.taint_subject field replaced by payload: CheckPayload (accessor taint_subject()). sort_diagnostics debug_asserts the contract. Wire: Finding::structured_data now always emits CheckPayloadData for compiler checks; the old process-local handle moves to field sourceAdvice, and ContextDiagnosticData::from_value unwraps it.

5. DEAD ENDS
- Only inserting the namespace into `namespaces` is not enough; registry command paths also need world geometry.
- Looking for a fix in matches_original_procedure_summary: it passes; the cause was may_calls None from original_head.
- End-to-end fixtures for O106/T106/W201/IRULE4002/4004: none fire on work (tried many sources).

6. NEXT STEPS
1) git fetch; rebase issues/generations-payloads onto origin/work; rebuild.
2) Fresh baseline worktree at the same origin/work; full compiler + adapter suites both sides; require 0 new.
3) Rerun clippy + 5 gates; git push origin HEAD:work (rebase/retry if rejected; never force work).
4) Comment and close #2426 and #1797 (state_reason completed), comments end with the Generated by Claude Code footer.
Pitfalls: 4 cores / 15 GB: one cargo build at a time (a test run alongside is fine). Compiler suite ~25-40 min. Disk got to 11 GB free with two target dirs; delete the worktree target after. Debug builds need CARGO_PROFILE_TEST_DEBUG=0. tcl-mcp tests need --bins. tclsh8.4..9.1 are in ~/.local/bin. work moves fast (two rebases needed in a few hours).
```
