# Hand-over: iRules and taint barriers (#1796 witnesses, held taint work for #1710)

Written 17 November 2026 by the outgoing session, paused since 16 November, as a message to the orchestrator and filed here unedited. Its #1796 part goes to the `dataflow` issue session; its taint part goes to the burn-down's `taint` session.

The paths and commits below predate the file split of 16 November: see *Starting* in `../README.md` before applying anything.

```text
HANDOFF — session_017u5g3KRpbHebfAxzbMbnAe. Nothing unpushed; tree clean.

1. ISSUES
- #2528 CLOSED (fixed on work: 7d458dd6 parser plumbing, fb83bbef hosted advisory route). Close comment 6899260648. Open follow-up reported by the taint/iRules session (session_017u5g3HxGMgE8DmBYQoVvrC, 2026-11-16): under f5-irules a BRACED `${addr}` inside a template word (`set msg "src=${addr}:80"` in a `when HTTP_REQUEST`) has no read reference: SsaSourceView::read_reference(site,"${addr}") is None for both the access span and the component span, and read_spelling is None; unbraced `$addr` works. The access *is* in tokens.variable_accesses with original_spelling "${addr}". Suspect `hosted_original_substitution_access` (command_binding/original_variable_compilation.rs) records `$name` but not `${name}`. It breaks 4 it_3 taint tests: crlf_free::crlf_free_survives_safe_concat, crlf_free::interpolation_without_crlf_preserves, interpolation_colour_invalidation::crlf_free_preserved_without_literal, non_dash_prefixed_interpolation::literal_dash_prefix_warns. Not started.
- #1796 OPEN. On work: 825543a4 (typed FrameTarget/distance-aware upvar/uplevel) and fc797eb9 (single owner cfg_builder/frame_effects.rs::detect_frame_effect_bodies → FrameEffectSummaries{caller,global}). Comments 6894819908, 6899380975, 6900310884. Remaining: the precision witnesses (a). These wait for the dataflow session's #2408 commit. Then rebase, add the two witnesses (`deep`'s direct caller is left untouched; `w2`→`w1`→`outer` closes exactly to `outer`), verify on 8.4–9.1, and close.
- #2151 / #2165 DEFERRED (need a BIG-IP appliance measurement). Probes are committed under scripts/dev/bigip-probes/irules/. Comments 6900424707, 6900425069. Do not work on them.
- #1710 taint barriers (my sub-task; I have NOT posted on #1710). State below.

2. BRANCHES
- work @1ebec1f5 contains all the #2528 and #1796 commits above.
- issues/taint-barriers (only here, not on work):
  - 57386dde "Check a concatenating eval's words where they reach the wrapper". The fix: the taint sink scan skipped synthetic EvaluatedArguments boundaries. Multi-word `eval p $x s` lowers as a proved wrapper (EvaluatedArguments / phases / EvaluatedWrapper), so T100 never fired. New CommandTokens::evaluated_arguments_invocation() (ir.rs); used in taint.rs emit_statement_warnings and in registry_invocation/source_taint.rs original_source_taint_read_operands (the parent==tokens check). Includes witness test t100_fires_for_a_tainted_word_of_a_concatenating_eval_on_every_release.
  - 02f66c9c WIP, NOT BUILT/RUN: moves 10 dialect-blind taint::tests onto new helper release_registries() (tcl8.4..9.1 projected) and folds the witness into t100_span_is_the_tainted_argument_word_not_the_whole_statement (adds `eval [list puts $x]` negative). The 10: a_tainted_name_an_opaque_switch_arm_may_overwrite_stays_tainted, point_binding_supplies_sink_alias_prefix_once, t100_fires_for_tainted_while_and_for_conditions, t100_fires_when_nested_command_sub_is_itself_a_sink, t100_span_..., t101_cleared_only_by_a_mapping_that_deletes_crlf, t102_uses_ensemble_subcommand_label_and_t103_for_regexp, t103_cleared_by_an_inline_regex_quote_in_the_pattern_slot, t103_not_cleared_by_an_escaped_bracket_that_never_calls_the_quoter, taint_warnings_emitted_in_deterministic_proc_order.
- issues/frame-reach @0d72a433 and issues/irules @1a2501ed are fully merged into work.

3. VERIFICATION
- Env always: export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0. Dev profile only, never --release.
- Build: `cargo test -p tcl-compiler --lib --tests --no-run`. Binaries land under target/debug/build/tcl-compiler/*/out/ (NOT target/debug/deps): take the paths from the "Executable" lines.
- Run each binary with `--skip depth_guard::tests::the_source_walk_cap_fits_its_stack_budget` and `grep -E '^test .* FAILED$'` → failure list per binary.
- Baseline method: for the candidate, `git checkout <parent> -- <changed files>`, rebuild, run `<bin> --exact <names...>` for exactly the candidate's failures. Anything that passes there is NEW. Restore with `git checkout HEAD -- <files>`. Do NOT rebuild while a run is still executing binaries (the paths are reused).
- 57386dde was verified BEFORE the rebase (as 317c974f on 987bbfcb): 0 new failures. Failure counts on that tree: lib 1737, it_2 182, it_3 227, it_4 162, it_5 94, command_resolution_conformance 1, native_arithmetic_conformance 1, per_item_corpus 1; differential_incremental and resolution_rewrite_oracles pass.
- After the rebase onto 07fd5e6b: clippy (`cargo clippy -p tcl-compiler --all-targets -- -D warnings -A clippy::assert_is_empty`) clean; xtask registry-axes / value-transfers --check / owner-resolution / callback-inventory --check all OK; lib `taint` filter 218 passed / 71 failed with the witness passing. The FULL suite was NOT re-run after the rebase, so work is not fast-forwarded. Rerun the full suite plus the baseline method, then `git push origin HEAD:work` (fast-forward only).
- taint::tests at 987bbfcb: 35 failing; with the eval fix 34 (t100_for_a_tainted_switch_integer_subject recovered).
- rustfmt --edition 2024 on changed files.
- tclsh 8.4–9.1 are in /root/.local/bin. Verified: `eval "p" $x "s"` with x=`a; puts INJECTED; p` prints INJECTED on all five; `eval [list puts $x]` prints the text once.

4. DESIGN DECISIONS
- #2528 option (b): handler bodies stay advice-only, with a hosted-specific route (comment 6899260648).
- Taint barriers (1): the parent chose to move dialect-blind fixtures to named releases, NOT to design metadata for profile-less registries. Reason: no profile = no release, so failing closed is correct. Precedent: 25b904f6be, 0f9dd5c3e3. Recorded only in parent messages, not on GitHub.
- The eval fix treats the EvaluatedArguments boundary as the wrapper's own invocation, since that's where the substituted words reach eval. Phases and the EvaluatedWrapper boundary stay non-sinks.

5. DEAD ENDS
- Do NOT special-case profile-less registries in cfg_builder source_upvar_effects or head_is_unseen (skip barriers when registry.get(head) has no frame_effect). Tried it: `<unseen-call>` went away but `<caller-frame-opaque>`/`<global-frame-script>` stayed, and `set x [gets stdin]` still lowers as a plain Call with defs:[]. Root cause: OwnedInvocationMetadataContext::Standalone.metadata_context gives a None context when registry.profile() is None, so original_invocation_metadata_for_module returns None and the module command table names no builtins. Barriers were never the real blocker.
- `eval puts [list $x]` (multi-word) still gives T100 by existing design (list_wrapped_arg_command_is_literal requires a single script operand). `eval [list p $x]` with an unknown head `p` also warns. Neither is a regression.

6. NEXT STEPS (in order)
a. Build and run 02f66c9c's 10 moved fixtures. Any that still fail projected are real bugs: fix them, don't relabel. Drop "WIP" from the message, or split it into a clean commit.
b. Full suite + baseline for the branch, then fast-forward work. Message the audit session session_017u5g3Kfy8d2hR3BY5TKaZV with the SHAs. One progress comment on #1710.
c. The 7 callback-replay taint::tests (callback_list_replay_uses_the_selected_captured_builder_prefix, ..._static_expansion_owner, callback_replay_batches_past_the_first_batch_without_truncating_later_sinks, callback_replay_name_search_is_single_pass_under_collision_stress, callback_replays_batch_into_one_extra_interprocedural_build, command_substitution_callbacks_stop_before_replay_when_original_owner_is_refused, variable_trace_taint::dynamic_trace_target_taints_every_name maybe). Coordinate with the audit session (its replay change d9f49ecc).
d. About 15 projected-registry missing-metadata failures. Fix at the owner; don't hide the panic. Examples: actual_option_advice_... ("selected source sink schema"), alias_prefix_values_... ("assignment carrier"), taint_source_word_requires_... ("actual source statement tokens"), hosted_taint_diagnostics_... ("genuine conditional hosted sink"), branch_nested_sink_..., mixed_source_word_..., inline_sanitiser_..., constant_command_head_..., native_source_option_..., original_cross_interp_..., original_module_sink_candidates_..., actual_regexp_warning_..., irule3002_fires_in_command_sub_form, irule3101_pure_var_ref_..., irules_http_uri_is_a_dialect_agnostic_source, t102_fires_for_assign_value_wrapped_call, t102_span_is_tight_around_the_tainted_argument_assign_value.
e. #1796 witnesses after #2408. Then the braced-template-read gap above.
Pitfalls: the full lib run takes ~23 min, and a full suite ~1h20; run it in the background. Disk is a per-session allowance: on ENOSPC delete target/debug/incremental and stale build dirs. When rebasing after push retries, SHAs change, so report the final SHA. The cfg_builder unit fixtures are broken on work (pre-existing); use the it_4 analyser tests instead.
```
