# Outstanding work: compiler agent

Area: Lowering, C6 fresh builders, iRules RuleLoader realm, itcl/snit, OO.

Last updated: 2026-10-22T10:53Z (Sol-49)

Maintained by the compiler agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

Full change notes: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/sol-49-NOTES.md`.

## In progress

- **C6 zero-regression pass.** The tcl-compiler lib build carrying the following is running:
  - the `fp_rbs_15` fix: `analyser/bounds_checks.rs::script_words` reads switch clause-list action bodies, so a `break` there counts as a loop exit (tclsh: the loop ends)
  - the `fp_inj_*` fixture correction (into `when HTTP_REQUEST`)
  - the `compilation_unit` hosted-entry test correction
  - the CRLF fix in lowering `guarded_body_text`
  - a temporary probe for `fp_sh_21`, to be removed before hand-back
- **`fp_sh_21_dict_literal_in_dict_for_silent`.**
  - Cause so far: the module's `deferred_writes.any` becomes true when the `dict for` body holds `puts "$k=$v"`. The empty body gives false.
  - With every variable externally mutable, SCCP is all Overdefined and S100 fires.
  - The probe narrows which `deferred_writes/supplied.rs` step fails for the body statement: retained tokens, footprint, source purpose or callback invocation.

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
