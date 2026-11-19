# Analyser binding at the 19 November 2026 pause

All jobs owned by this session have stopped. No independent push or pull request was made. The coordinator integrated accepted changes locally on work. Recheck the actual fresh origin/work before applying anything.

## Wrong answers

- O101 remains a reproduced product error. On common head `8d6ae9adde1a1556b34a01952bd785fe972f9e69`, Tcl 8.6 produces no/yes from two successive calls in the it_2 witness. Its rewrite gives no/no. Experimentally preserving operator-error edges and requiring affirmative existence in the actual declared frame are separate changes below. The former earlier regressed it_5 recursive call diagnostics. Run an A/B for each and together. In compiler it_2 select `value_transfer_witnesses::try_and_errors::a_body_that_ends_in_an_error_may_have_failed_at_its_start` and `var_escape_cfg::an_operator_that_may_raise_reaches_the_try_handler`. Include it_5 `checks::commands::recursive_call_command_world::a_world_preserving_recursive_call_keeps_later_diagnostics`, plus lib controls ending `info_exists_fold_surfaces_o101` and `info_exists_rewrite_requires_actual_selected_entry`.
- O103 has a candidate integrated at `9a0dc3fbd3`. Assessment before the fix used a host entry that renamed ::answer and installed a procedure returning 99. All five C rewrites retained the call and gave 99. Jim gave 99 before and 42 after rewriting. The no-op external entry gave 42 before and after on all six. The captured conditional binding remains available for queries. A public source edit requires a reached target or an invariant across entry worlds, per ruling `1d77a99b70`. Verification after integration remains open. Repeat the six public comparisons and the compiler-lib tests ending `o103_scoped_call_withdraws_later_callee_changes_and_unknown_entry` and `o103_cmd_subst_fires_on_set_value_target`. Keep the proved after-load positive. See [pre-fix public comparison](evidence/analyser-c989-o103-external.json).
- See [class-facts](class-facts.md) for stale globals, the renamed class under catch and TclOO tests. Namespace-unknown and rename-trust historical patches and controls are retained in the [previous handoff](../pause-2026-11-18/analyser-binding.md).

## Declared-frame collector

Candidate `285257233f` replaces the positional merge experiment at issues/dataflow-2 `429ef5da`. Associate each recorded point and source read with the evaluating declaration origin, span and frame. Preserve both reused and new entries, including a true remote uplevel #0 row under its declaring root. The old branch contains local probes and a global measurement switch. Neither its raw offset complement nor its append-only index partition proves frame ownership.

The clean compiler baseline at `a6c66517c7` passed all 65 preservation controls and failed all six intended recovery controls. All six interpreters printed `first second shared` for the frame fixture. The collector candidate has not been compiled or run against these controls. The LSP lib control `refactor::inline_variable::tests::inline_keeps_the_following_line_indentation` is a separate 72nd control and remains unmeasured at this head. Rerun the exact compiler selectors and the new `command_binding::tests::source_walk::a_declared_body_keeps_its_own_local_and_remote_frame_evidence` before enabling the merge. [Named clean results](evidence/analyser-collector-clean-a6c66517c7.tsv) · [Filter](evidence/analyser-collector-nextest.filter) · [Oracle fixture](evidence/analyser-collector-frame.tcl)

## Parser migration

The integrated changes include the tcl-expr crate with forwarding syntax APIs, selected GVN grammar, shared formal parameters, retained array read tracing and original argv projection for uplevel. The call-by-name candidate below also inventories original nested calls and declines incomplete O109/O126 edits. The fixture produced `7 8 14` on all six for grouped names, brace-protected dollars and an unusual default formal. Resolve its two standalone `LexerConfig::for_profile(None)` calls and check the codegen it_3 cell-default control before integration. [Oracle fixture](evidence/analyser-call-by-name.tcl)

The unpublished insertion issuer checks reached original argv, namespace head, dialect, execution realm, observers, C ensemble private worker mapping and Jim's native dict routes. It supplies prospective exact switch, dict get/getwithdefault, return and set claims for lsp-features. Tests for the real public rewrite, private-worker substitution, negative entry and selected release remain outstanding. Dict get is absent in C 8.4. Getwithdefault is available on C 9 and Jim. [Oracle fixture](evidence/analyser-insert-commands.tcl)

The 39 individual parser sites and seven repeated calls were reviewed. The larger structural inventory has **1,665 open site dispositions**. Of those, 1,663 matched enclosing Rust functions and two marks referred to the removed parse_uplevel_level function. AST mapping alone does not settle a row. Examine condition_constant, const_bool, coerce_scalar and constant_index for selected host-number parsing. Internal Rust format %d parses 010 and 08 as 10 and 8 for every version. C before 9 gives 8 and an error. C 9 and Jim give 10 and 8. The 48 prebuilt c989 public scenarios did not expose a source miscompile. Dataflow owns removal of the independent SCCP format route. [39 sites](evidence/analyser-parser-sites.tsv) · [Seven repeated calls](evidence/analyser-repeat-sites.tsv) · [Public format comparisons](evidence/analyser-c989-format-d.json)

## Patch inventory

Each source patch is experimental. The listed parent identifies its exact standalone diff. Check for already integrated work and inspect changes before rebasing.

| Patch | Exact parent | Candidate |
| --- | --- | --- |
| [analyser-o101-operator-edges.patch](patches/analyser-o101-operator-edges.patch) | `e646046f2fcdd5188e2d0daff270bbd566899405` | `261b6cb0a791462b1d75ff580aea775f1938fd2e` |
| [analyser-o101-rewrite-entry.patch](patches/analyser-o101-rewrite-entry.patch) | `33ad9ad5069c5594c01b0449c92efda611f1d084` | `60258f6c4181a01be4efc87c6a421b0a8c7d463f` |
| [analyser-collector-declaration-origin.patch](patches/analyser-collector-declaration-origin.patch) | `d9b4c677e6df42d44d76d907881f9cacd40760e6` | `285257233f4f34948a9da880645607cf3e3f6594` |
| [analyser-call-by-name-inventory.patch](patches/analyser-call-by-name-inventory.patch) | `8c6cb58e46b696cb00fbbd7adcbf92355aca8131` | `c631bbab7e89b84d7a209b4390e656ce79255453` |
| [analyser-comparison-insertion-proof.patch](patches/analyser-comparison-insertion-proof.patch) | `ae27219e1dd262ffe2d998470cf8e235a36be222` | `0b3386b24ca3608932dc14d33a8313574d9517fb` |
| [analyser-existing-rustfmt.patch](patches/analyser-existing-rustfmt.patch) | `2987829759360e4526e30809bbf19bc066457347` | Working-tree formatting only, already visible on coordinator work |

## Restart measurement

Use a fresh origin/work checkout on branch work. Rebuild Jim using `bash scripts/dev/ensure-jim-oracle.sh` and select all six executables. In this environment source `/workspace/tcl-lsp/tmp/oracle-env.sh`. Export `TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1` and measure clean HEAD before quoting new counts. The clean compiler baseline at a6c66517c7 was 11,289 pass, 1,871 fail and 6 skipped. All 404 tracked analyser failures were still present. The clean registry had 2,467 passes. Coordinate the large Rust build slot with diagnostics and execution, then run cargo fmt, clippy --all-targets and make xtask-gates before every push.
