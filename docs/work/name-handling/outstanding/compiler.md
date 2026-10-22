# Outstanding work: compiler agent

Area: Lowering, C6 fresh builders, iRules RuleLoader realm, itcl/snit, OO.

Last updated: 2026-10-22T12:29Z (Sol-49)

Maintained by the compiler agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

Full change notes: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/sol-49-NOTES.md`.

## Resume

Evidence directory (in the repo): `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler/`.
- `scripts/` holds the batch scripts. Set `SOL49_SCRATCH` to a scratch directory first; results go to `$SOL49_SCRATCH/res/<tag>-<filter>.txt`.
  - `cycle2.sh <tag> <filters…>`: one command that builds the `tcl-compiler` lib tests, runs the `zz_sol49` probes with `--nocapture` (written to `<tag>-probe.log`), runs each filter, then deletes the executable.
  - `cycle.sh`: the same, without the probes.
  - `cmp.sh <base.txt> <after.txt>`: prints pass-to-fail tests and the gain count.
- `sol49-results-z2-y3-d1.tar.gz`: per-test ok/FAILED lists. `z2`/`y3` are the pre-C6 baselines; `c1`/`d1` are with C6.

Where to pick up:
1. Running now: cycle `e1`. Command:
   `THREADS=3 cycle2.sh e1 fp_rbs_15 fp_sh_21 fp_tnt_03 irules_checks:: analyser::diagnostics::tests registry_invocation signature_scan:: taint:: lowering:: source_class::tests depth_guard::tests::cap_deep depth_guard::tests::deep_namespace_eval_lowering cfg_builder:: deferred_writes command_binding::`
   Compare each filter with `cmp.sh res/z2-<filter>.txt res/e1-<filter>.txt`; use `y3` where `z2` has no list for that filter.
2. Use the `e1-probe.log` output:
   - `ZZ_SOL49_ANY supplied.rs:<line>` lines show which site turns `deferred_writes.any` on for `fp_sh_21`.
   - The `FU` block dump shows the CFG for the W210-after-switch cases (the `fp_rbs_15` trio).
   - `zz_sol49_hover_schema_probe` shows which `source_registry_words` stage withdraws the schema for `regexp {plainword} $line`.
3. Remove the temporary probes before hand-back:
   - `zz_sol49_dict_for_probe` in `analyser/diagnostics/fp/sh.rs`
   - `zz_sol49_hover_schema_probe` in `registry_invocation/source_structure.rs`
   - `zz_sol49_any` in `deferred_writes/supplied.rs` and `deferred_writes.rs`: restore every `zz_sol49_any(line!())` to `true` and delete the function.
4. Then the integration binaries, as one build-run-delete command each: `cargo test -p tcl-compiler --test optimiser_coverage`, and `cargo test -p tcl-spectcl --test spec_corpus shared_unit_matches_legacy_for_representative_profiles`.

## In progress

- **C6 zero-regression pass.** The full tcl-compiler batch (build, run and delete in one command) is running with these fixes:
  - **`fp_rbs_15`** (two parts):
    - `bounds_checks::script_words` reads switch clause-list and inline action bodies.
    - `cfg_builder/cfg_lower.rs::lower_opaque_switch` wires an opaque switch's `break`/`continue` to the enclosing loop before treating an all-exiting switch as a procedure exit. This also targets the earlier-failing `fp_rbs_15_all_break_switch_not_a_proc_terminator`.
  - **`fp_sh_21`.** `deferred_writes/supplied.rs` no longer turns `any` on for a nested receipt that is already a lowered statement of the command's own body script. The probe showed that `puts` inside a `dict for`/`foreach` body set `deferred_writes.any`, so SCCP went all Overdefined.
  - The `fp_inj` and `compilation_unit` corrections, and the CRLF lowering fix.
  - Partial results of that batch (d1, against the pre-C6 baseline z2): `diagnostics::fp` 484 ok / 97 failed, with 3 pass-to-fail left (`fp_rbs_15_while1_break_in_opaque_switch_exits_loop`, `fp_sh_21_dict_literal_in_dict_for_silent`, `fp_tnt_03_interp_eval_list_literal_head_no_t105`) and 40 gains. `interprocedural` 76/51 with 0 regressions. `bounds_checks` 56/13. `diagnostics::helpers` 13/9. The `supplied.rs` change did not clear `fp_sh_21`; a probe build follows the batch.

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
   - N08: `inline_uplevel` existence oracle and nested `lower_literal_script` context
   - N09: `type_infer` constructor lookup is global-first; interprocedural/taint resolution ignores `namespace path`
   - N10: superclass/mixin identity is rebuilt from the class owner, plus the unique-tail rescue
   - §3.3: `optimiser::method_barrier` global-first / first-tail class lookup

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
