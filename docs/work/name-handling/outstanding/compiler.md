# Outstanding work: compiler agent

Area: Lowering, C6 fresh builders, iRules RuleLoader realm, itcl/snit, OO.

Last updated: 2026-10-22T13:12Z (Sol-49, paused on the user's instruction)

Maintained by the compiler agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

Full change notes: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/sol-49-NOTES.md`.

## Resume

**Paused at 13:10Z on the user's instruction.** Nothing of mine is running. The e1 test executable is deleted.

Evidence directory (in the repo): `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler/`.
- `scripts/`: the batch scripts. Set `SOL49_SCRATCH` to a scratch directory first; results go to `$SOL49_SCRATCH/res/<tag>-<filter>.txt`.
  - `cycle2.sh <tag> <filters…>`: one command that builds the `tcl-compiler` lib tests, runs the `zz_sol49` probes with `--nocapture` (with `ZZ_SOL49_ANY=1 TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS=1`, written to `<tag>-probe.log`), runs each filter, then deletes the executable.
  - `cycle.sh`: the same, without the probes.
  - `cmp.sh <base.txt> <after.txt>`: prints pass-to-fail tests and the gain count.
- `sol49-results-z2-y3-d1.tar.gz`: per-test ok/FAILED lists. `z2`/`y3` are the pre-C6 baselines; `c1`/`d1`/`e1` are with C6.
- `e1-probe-summary.txt`: the probe findings. `e1-probe.log.gz` is the full log.
- `zz_sol49_probes.rs.txt`: copies of the two inline probe tests.

Partial C6 batch results, against z2:
- d1:
  - `diagnostics::fp` 484 ok / 97 failed: 3 pass-to-fail, 40 gains.
  - `interprocedural` 76 ok: 0 pass-to-fail.
  - `diagnostics::helpers` 13 ok.
  - `compilation_unit` 36 ok: 0 pass-to-fail.
  - `irules_event_checks` 77 ok: 0 pass-to-fail.
  - `bounds_checks` 56 ok.
- The coordinator pruned the d1 executable after `irules_event_checks`, so the later d1 filters ran no tests.
- e1 completed the build, the probes and `fp_rbs_15` (6 ok / 3 failed, unchanged), then stopped for the pause.
- Not yet measured with C6: `command_binding::`, `irules_checks::`, `diagnostics::tests`, `registry_invocation`, `signature_scan`, `taint`, `lowering`, `source_class`, `depth_guard`, `cfg_builder`, `deferred_writes`, and the integration tests `optimiser_coverage` and `spec_corpus` I230.

Probe findings (e1), to act on next:
- **`fp_rbs_15_while1`.** In the procedure CFG, the loop body block `B2` (holding the switch) only goes back to the header `B1`; nothing wires it to the exit block `B3`. So the switch's `break` never reaches `lower_opaque_switch`'s escape wiring: either the switch is not lowered as an opaque switch inside the `while 1` body, or `switch_escaping_jumps_with_classes` returns false. Check `cfg_lower.rs` along the path from the `while` body to the switch statement.
- **`fp_sh_21`.** `deferred_writes.any` is set in `deferred_writes/supplied.rs::original_registration` at three points:
  - `with_metadata_schema` / `selected_values` returns None (the instrumented line 148, most frequent);
  - no `original_materialized_footprint_for_module` (line 118);
  - `deferred_namespace(value.scope)` is None (line 154).

  Next step: log the command name at each of these points.
- **`fp_tnt_03`.** With an unset `$child`, the TN `interp eval $child [list puts $raw]` still emits T105. The nested `[list]` invocation is unreachable, so `taint.rs::list_wrapped_arg_command_is_literal` gets no proved wrapper invocation. With a created child (`set child [interp create]`), the TP `interp eval $child [list $raw]` emits nothing at all, which is a real FN.
- **hover_residual.** At compiler level the schema is NOT withdrawn. For `regexp {plainword} $line` (with `$line` undefined), `source_registry_words` gives words and `with_source_schema(...).authored_source_pattern_arguments()` gives 1 pattern; `lsearch -exact $list $needle` also gives words. The withdrawal must be on the LSP side (`tcl-lsp-core` `hover.rs` / `original_invocation.rs` walk, or the registry snapshot-key check). That is outside my files.

Exact next command, once resumed and the disk is above 2.3 GB (it reruns the measurement; add `interprocedural:: analyser::diagnostics::fp` after any code change):
`SOL49_SCRATCH=<scratch> THREADS=3 docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler/scripts/cycle2.sh e2 fp_rbs_15 fp_sh_21 fp_tnt_03 command_binding:: irules_checks:: analyser::diagnostics::tests registry_invocation signature_scan:: taint:: lowering:: source_class::tests depth_guard::tests::cap_deep depth_guard::tests::deep_namespace_eval_lowering cfg_builder:: deferred_writes`

Temporary code to remove before hand-back (all of it compiles):
- `zz_sol49_dict_for_probe` in `rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs`
- `zz_sol49_hover_schema_probe` in `rust/tcl-compiler/src/registry_invocation/source_structure.rs`
- `zz_sol49_any` in `rust/tcl-compiler/src/deferred_writes/supplied.rs` and `rust/tcl-compiler/src/deferred_writes.rs`: restore every `zz_sol49_any(line!())` to `true` (and the two `&& zz_sol49_any(line!())` operands), then delete the function.

Files I have touched since e34d8a427e (details in `sol-49-NOTES.md`):
- `ir.rs`
- `lowering/mod.rs`
- `depth_guard.rs`
- `source_transition_advice/source_class.rs`, `source_transition_advice.rs`, `source_procedure.rs`
- `command_binding.rs`: re-export and C6 hunks
- `realm.rs`, `realm/source_transition_advice.rs`
- `registry_invocation/source_structure.rs`
- `analyser/original_members.rs`, `analyser/handlers.rs`, `analyser/irules_event_checks.rs`, `analyser/bounds_checks.rs`
- `cfg_builder/cfg_lower.rs`
- `deferred_writes/supplied.rs`, `deferred_writes.rs`
- `compilation_unit.rs`
- the C6 companion files
- `fp/inj.rs`, `fp/sh.rs`
- `docs/design/contracts/naming-consumer-matrix.md`

## In progress (paused)

- **C6 zero-regression pass.** Landed in the tree: the companion and the WIP hunks, plus:
  - the `bounds_checks` W241 switch-clause fix;
  - the `cfg_lower` opaque-switch loop-escape ordering;
  - the `supplied.rs` lowered-child skip;
  - the `fp_inj` and `compilation_unit` corrections;
  - the CRLF lowering fix.

  Three `diagnostics::fp` pass-to-fail remain (see the probe findings above).
- **Classifying compiler-names' 8 `command_binding::` pass-to-fail** (`compiler-names/g1-command_binding-pass2fail.txt`). Started by reading the tests; not run yet. First reading:
  - `expression_preparation` ×2, `lifecycle_advice`, `normal_result`, `declaration_layout`: they assert that an UNCALLED `proc` body has no proved execution target. The user decision of 2026-10-21T08:20Z ("world after load") makes those commands resolve, so these are likely (a), each needing a per-assertion correction that cites the decision.
  - `conditional_expression` "Reached": comes from `walk_deferred_entry_with`, which sets `declaration_preview_depth` to 0 for an after-load entry, so `runtime_reachability` becomes Reached (`command_binding.rs` around line 3052). With preview depth 0, the uncalled body also records actual compiler visits and failures and marks called implementations. That needs a decision: keep Reached as the modelled after-load call, or add a separate reachability. Not settled.
  - `original_body_namespace`: comes from the C6 `record_conditional_in_frame_script` hunk. A conditional in-frame `if` body now reports the namespace it would run in (`::`); likely (a), with the correction `Some(expected)`.
  - `source_command_world` import: `original_completed_command_world()` is None. That is compiler-names' R1 area, but it may be C6's fresh entry (`rename auto_import`); needs a run.

## Outstanding (in priority order)

1. Finish C6. After the running build, run the full tcl-compiler filter batch:
   - `interprocedural`, `diagnostics::fp`/`tests`/`helpers`
   - `compilation_unit`, `irules_*`, `registry_invocation`
   - `signature_scan`, `taint`, `lowering`, `command_binding`, `depth_guard`, `source_class`

   Then the integration binaries: `optimiser_coverage`, and `tcl-spectcl` `spec_corpus` `shared_unit_matches_legacy_for_representative_profiles` for I230. Zero pass-to-fail, or a documented correction.
2. `fp_tnt_03_interp_eval_list_literal_head_no_t105` (C6 blocker).
   - Under the fresh interpreter `$child` is unset, so the nested `[list …]` invocation is never reached and its suppression doesn't apply.
   - Creating the child in the fixture made the TP sibling (`…_tainted_head_fires_t105`) lose T105. That is a real FN for a created interpreter, so I reverted the fixture change. Needs a fix in the T105 sink or suppression.
3. Hand-back gates for my files:
   - `cargo fmt --check`
   - `cargo clippy -p tcl-compiler`, changed lines clean (the `derived_hash_with_manual_eq` on `ir::Script` is already fixed)
   - `cargo check --workspace --all-targets`
   - remove the temporary `zz_sol49_dict_for_probe` test
4. IRULE4001/6001 in hosted procedure and event bodies (`source_graph::ordinary_rule_init_reporting_name_*`, `irule4001_fires_for_static_write_outside_rule_init`, `irule6001_*`). The body's `set ::static::x` has no original variable operands, so `NormalTransferInvocation::mutation_places` returns `unknown_top`. These were already failing before my iRules work.
5. Review findings, pending triage routing:
   - The triage report `docs/work/name-handling/reviews/naming-migration-review-20261022-triage.md` (12:43Z) lists N08 and §3.3 `method_barrier` as FIXED.
   - Still open, not yet routed to me:
     - N09: constructor typing is fixed, but `::n::c` gets no call-graph summary.
     - N10: a class selected through `namespace path` still gives no hover, type hierarchy or definition.

## Blocked (on whom, and what is needed)

- **itcl contract-D extension.** Affects 4 tcl-lsp-core `name_resolution` `itcl_class_proc_dispatch` tests: `tp_every_dispatch_spelling…`, `tp_references_from_the_call_site…`, `fn_to_tp_the_class_resolver_pre_empts…`, `fp_a_shadowing_proc_wins…`.
  - (a)/(c) In-namespace class declarations and calls after them: the advice graph treats every `namespace eval` body as a possible branch (`source_transition_advice/body_effects.rs::retain_possible_effects` only records possible occupants). That is compiler-names' file, and the change is design-level: install a definitely entered body's publications in the parent graph. Needs an owner decision. TclOO ctor calls after a namespace-eval class have the same limit.
  - (b) Calls inside itcl member bodies (`[make]` in `proc other`, `[Factory::make]` in `method inst`) need a new advice walk of itcl member bodies under the class-namespace resolver.
- **LSP `name_resolution` tests outside my items:** `namespace_scoped_class_dispatch::{fn_to_tp_an_explicit_namespace_import_is_followed, tn_object_command_dispatch_is_scoped_to_its_creation_namespace}`, `pkgindex_dir::fp_other_undefined_var_in_pkgindex_still_fires`.
- **Compiler-names R1 completed-world change.** 10 `command_binding::` tests assert `original_completed_command_world().is_none()` and now fail. Routed to compiler-names.

## Done since the last update (with evidence: test counts, commit or snapshot)

- **Compiled-word miscompile.** `string length "{}$z"` now gives 3. Test `native_operand_words_in_a_substitution_keep_their_variables_live`, tcl8.4–9.1.
- **Stack overflow at the structured depth cap.**
  - Segmented-stack `Script` Clone/PartialEq, manual Hash.
  - Take/borrow instead of clone in `extract_oo_methods_pass` / `populate_trace_facts`.
  - `lower_command` split: about 29 → 18 KiB per level.
  - Tests: `depth_guard::{cap_deep_structured_if_ir_clones_and_compares_within_the_budget, deep_namespace_eval_lowering_fits_the_budget}`.
- **Contract D.**
  - snit typemethod is now a ClassObject member.
  - New `source_class_proc_call_at` / `OriginalSourceClassProcCall` for itcl `Class::proc`, with shadowing.
  - The itcl two-word form stays construction.
  - LSP: `tp_snit_typemethod_bare_dispatch_is_a_reference`, `fn_to_tp_itcl_class_proc_colon_dispatch_is_a_reference` and `fp_itcl_bare_two_word…` pass.
- **iRules RuleLoader realm.**
  - `proc` is an original declaration with an original lookup (`handlers.rs` declaration policy, `realm.rs` hosted document realm).
  - The IRULE5001 `static::` gate is spelling-aware.
  - tcl-lsp-core `name_resolution` is 70/77; `irules_switch_glob_body_is_followed` gives `[0]`.
- **C6 applied:** the companion plus `sol-49-c6-wip.patch`: after-load, condition slot, conditional in-frame body namespace. Measured without the realm change:
  - `interprocedural` 55 → 76 (0 regressions)
  - `diagnostics::helpers` 5 → 13
  - `diagnostics::fp` 447 → 479
  - remaining blockers are listed above
- **Pre-C6 batch:** 0 pass-to-fail from my changes across `irules_event_checks`, `irules_checks`, `diagnostics::tests`/`fp`, `registry_invocation`, `signature_scan`, `taint`, `lowering`.

## Coordinator note (2026-10-22T13:30Z)

- The "reached" question is decided (`user-decisions-20261022T1317Z.json`): analysis models the code once loaded and running, so for after-load entries the preview-depth-0 walk that records actual compiler visits stays the modelled call (no separate reachability value). Apply narrow test corrections to the 8 `command_binding::` tests where they contradict this, citing that decision and the 2026-10-21T08:20Z decision; ask the user about any case that does not fit.
- hover_residual: the schema is lost on the tcl-lsp-core side (`hover.rs` / `original_invocation.rs`), so it belongs to the lsp role (noted in outstanding/lsp.md).
- Review N09/N10 residuals (no call-graph summary for `::n::c`; a class selected through `namespace path` gets no hover, type hierarchy or definition) are routed to compiler.
