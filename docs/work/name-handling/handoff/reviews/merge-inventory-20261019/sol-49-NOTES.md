# Sol-49: compiled-word miscompile, iRules fresh rule load, lowering stack depth, contract D

Implementer: Sol-49. Branch `work`, base `e34d8a427e`. I did not commit, push, stash or reset; the coordinator's WIP snapshots (363a9f89de, 6afd791002, …) carry my edits as they stood. These notes cover every change I made since `e34d8a427e`.

Owned paths: `rust/tcl-compiler` (except the files handed to compiler-codegen and compiler-names), `rust/tcl-registry`, `rust/tcl-syntax`, `docs/design`.

## 1. Compiled-path miscompile `string length "{}$z"` (task 1): fixed

- **Cause.** The VM's `subst_word` substitutes only `${…}` and `[…]`; a bare `$` is data. `emit_native_argument_word` took the original word whenever one existed. It then pushed the raw text, so `"{}$z"` was frozen as the literal 4-byte word `{}$z`. tclsh gives 3.
- **Fix** (`codegen/cmd_subst.rs`, now owned by compiler-codegen). Use the original word only when its legacy text equals the written text, or, unbraced, when both parse to the same substitution template. Emit it through `emit_word_from_source(word.legacy_text(), …)`, which normalises `$z` to `${z}`; otherwise fall back to `emit_cmd_subst_arg`.
- **Regression test** (`compile_service.rs`): `native_operand_words_in_a_substitution_keep_their_variables_live`, for tcl8.4–9.1. It covers `"{}$z"`, `"$z{}"`, `"a$z"`, `"a{}$z"`, `"\{}$z"`, `"\{\}$z"`, `string index "{}$z" 2` and `list "{}$z"`. Each case checks that no literal equals the written word and that STR_CONCAT1 is emitted.

## 2. iRules fresh entry, "realm-aware command world" (task 2, decision 2026-10-21T00:10Z)

- **`SourceAnalysisEntry::for_fresh_hosted_source`** (`command_binding/source_analysis_entry.rs`). A hosted F5 document with no supplied entry is a fresh rule load:
  - `unknown_entry = false`, realm `RuleLoader`.
  - `NativeRecipe` naming under the authored F5 Tcl 8.4 simulation.
  - `Direct` native compilation at the top level.
  - `for_supplied_source` falls back to it.
- **`BindingBaseline::hosted_rule_loader_world` / `body_compilation`** (`command_binding.rs`, `future_bodies.rs`): each entered `when`/procedure body selects TMM `Direct` compilation.
- **Formal topology and vendor catalogue advice** accept the authored simulation policy.
  - `vendor_catalogue_advice.rs`: catalogue advice answers only where the original table has no candidate.
  - `registry_invocation.rs`: the hosted assistance branch returns the original candidates first.
- **Registry**:
  - `commands/irules/proc.rs`: `procedure_definition: Core`.
  - `registry.rs`: the native return-state effect for F5 Tcl 8.4.
- **iRules `proc` as an original declaration, with its original lookup.**
  - `analyser/handlers.rs::declaration_name_policy`: for a hosted document analysed without a supplied entry, the declaration name policy is the fresh rule-load `NativeRecipe` policy. So `proc hit` appears in `original_procedure_declarations` (`::hit`, name span 5..8).
  - `realm.rs::document_vendor_realm_with_resolved_input`: the analyser's hosted document realm takes the same fresh rule-loader entry (realm, naming policy, `Direct`). So the loader's `proc` invocation keeps an `original_lookup`, which LSP selection at the declaration needs.
  - Result: tcl-lsp-core `name_resolution` `my_method_dispatch::irules_switch_glob_body_is_followed` gives `[0]`. The whole binary is 70/77; the LSP agent's last report was 65/77.
  - `analyser/irules_event_checks.rs`: `irules_debug_gate_opens` also accepts a written `static::` variable spelling (`names_static_variable`). It is suppress-only, like the existing `mentions_static_variable`. Under the entered event frame the relative name's namespace is not proved, and without this the realm change regressed 6 IRULE5001 `$static::` gate tests.
  - Tests:
    - `irules_event_checks::rule_procedure_is_an_original_declaration_of_the_fresh_rule_load`
    - `irules_event_checks::rule_loader_proc_invocation_keeps_its_original_lookup`
  - The three `irule5001_quiet_for_flag_set_in_*` failures were already failing in my pre-iRules baseline (r0).
- **Documented test corrections**, each supplying the explicit unknown-history entry the test is about:
  - `registry_invocation::hosted_original_source_assistance_keeps_metadata_and_execution_uncertainty_separate`
  - `irules_checks` (`supplied_with_history`)
  - `hosted_procedure_body` (`fixture_with_history`)
- **Design doc.** The `naming-consumer-matrix.md` row "Supplied source entry correspondence" documents the fresh rule load.
- **`irules_switch_glob_body_is_followed`** (LSP test; the call site is not a reference). BIG-IP evidence `scripts/dev/bigip-probes/resolution-2286`, case `ordinary_literal`: a direct literal call `same` to a rule-local `proc same` is rejected by the 21.1.0.1 loader with `undefined procedure: same` (also KCS IRULE5005). So `hit` is not a reference: the documented correction is `[0]`, which the LSP agent applied.

## 3. Lowering and IR stack depth (queue item 2): fixed

- **`ir.rs`: segmented-stack Clone and PartialEq.** `Script` no longer derives `Clone`/`PartialEq`. Manual impls run each nested script on a fresh 8 MiB stack segment every 8 levels (`with_structured_ir_stack`). `Hash` is manual too: it covers the same fields and stays on the caller's stack, since `Hasher` need not be `Send`. That fixes the deny-level `derived_hash_with_manual_eq`.
- **`lowering/mod.rs`**:
  - `extract_oo_methods_pass` takes and restores the scripts instead of cloning them.
  - `populate_trace_facts` borrows the bodies into `TraceFacts`.
  - `update_const_map` is `#[inline(never)]`.
  - `lower_command` is split. Its large selection frame moved to the out-of-line `lower_command_selection`, which returns `CommandLoweringSelection::{Lowered, Structured}`. Only the structured dispatch recurses. Nested `namespace eval` went from about 29 KiB to about 18 KiB per level (measured with gdb).
- **Tests** (`depth_guard.rs`):
  - `cap_deep_structured_if_ir_clones_and_compares_within_the_budget`: cap+8 nested `if`, lowered, cloned and compared on a 2 MiB thread.
  - `deep_namespace_eval_lowering_fits_the_budget`: cap+16 nested `namespace eval`, with a proc leaf and with a `set`/`puts` leaf, on the 1.5 MiB lowering share.
  - Earlier full-analysis versions also passed, but took 430 s and 172 s. Nested-`if` analysis cost grows about quadratically in a debug build: 16 levels took 11.7 s, 32 levels 54 s.
- **For compiler-codegen.** `compiled_preflight::CompilerTraversal` (`script` → `original_script` → `control_preparations` → `structured_compilation` → `command`) costs about 24.4 KiB per nested body level. That is above the 24 KiB per-level budget, and it overflows the 1.5 MiB lowering share at about 62 levels. It fits the whole 2 MiB thread.

## 4. Contract D: registry-dispatched definers (snit/itcl)

- **snit.** The type command was already published in the source advice world (`source_constructor_call_at` works for `Factory make`). The class-side member was missing: `typemethod` was recorded on the Instance side.
  - Fix (`analyser/original_members.rs`): a Registry type-object `Method` member (`MemberReceiver::TypeObject`) is declared on the ClassObject side, except for Itcl (below).
  - Test: `source_class::tests::snit_type_command_dispatches_its_typemethod_through_the_source_class`.
- **itcl `Class::proc`.** New `OriginalSourceClassProcCall` (`source_transition_advice/source_class.rs`), retained by `retain_source_class_proc_call` and hooked into `retain_original_source_receiver` before ordinary procedure calls.
  - A static `Qualifier::member` head counts when its qualifier resolves as a command name (current namespace, then global) to an Itcl source class.
  - The receipt exposes the class declaration, `member_name()` and `member_span()`.
  - Public query: `registry_invocation::source_structure::source_class_proc_call_at`, through the realm getter `original_source_class_proc_call`.
  - Test: `itcl_class_proc_calls_join_the_qualifying_source_class`, for `::app::Factory::make`, `app::Factory::make`, and a global `Factory::make` that must not resolve.
- **itcl two-word form.** `Factory make` constructs an object named `make` (Registry `conditional_construction_name_at`).
  - `display_constructor_shape` keeps BareWord for Itcl even when the word names a class proc.
  - Itcl class procs are not placed in the class-object two-word table, so the LSP's two-word query cannot select them. This also fixes the `fp_itcl_bare_two_word…` pass-to-fail the coordinator reported.
  - Test: `itcl_two_word_form_is_construction_even_over_a_class_proc_name`.
- **Shadowing.** A plain occupant (or possible occupant) at the qualified name itself wins: `source_class_proc_call_at` returns None for it. Covered in `itcl_class_proc_calls_join_the_qualifying_source_class`.
- **Not done (coordinator's 2a–2c):**
  - (a)/(c) In-namespace class declarations do not join from later top-level code. `body_effects.rs::retain_possible_effects` treats a `namespace eval` body as a possible branch and installs none of its cells into the parent graph. TclOO constructor calls after a namespace-eval class have the same limit. Fixing it is a design change in compiler-names' `body_effects.rs`.
  - (b) Calls inside itcl member bodies (bare `[make]`, `[Factory::make]` in a method) need the advice walk to enter itcl member bodies with the class-namespace resolver, which does not exist yet.
- **LSP follow-up (not mine).** `method_symbol::command_query` needs a branch for `source_class_proc_call_at`: select the class record by `class_declaration().matches_source_declaration`, then the member by name. Then `fn_to_tp_itcl_class_proc_colon_dispatch_is_a_reference` and `itcl_class_proc_dispatch::*` can pass. Inside-class-body and in-namespace spellings need the advice walk of those bodies; I have not verified them.
- **Design doc.** New `naming-consumer-matrix.md` row: "Registry-dispatched definer class commands (contract D)".

## 5. C6 (landed; remaining blockers listed)

- **Coordinator trace.** The I230 shared-unit versus legacy gap in `tcl-spectcl` `spec_corpus` is C6: a dialect-bearing `build_for_profile` must be a fresh interpreter. When I parked C6 I reverse-applied the companion, which removed `compilation_unit.rs::fresh_document_input`.
- **Applied, as the coordinator instructed:**
  - `sol-47-c6-companion.patch`
  - `sol-49-c6-wip.patch`, whole:
    - condition slot in `registry_invocation/original_word_data.rs`
    - conditional in-frame body namespace in `command_binding/origin_inventory.rs` and `command_binding.rs::walk_body_operand`
    - "world after load" for uncalled bodies (decision 2026-10-21T08:20Z) in `command_binding.rs`: `uncalled_body_after_load` and `walk_deferred_entry_with`, a Selected entry with no preview depth
  - The `command_binding.rs` lines are mine inside compiler-names' file, as coordinated.
- **Measured on a C6 tree without the realm change, against the pre-C6 runs:**
  - `interprocedural::` 55 → 76, 0 regressions (the four 1d blockers are resolved)
  - `analyser::diagnostics::helpers` 5 → 13, 0 regressions
  - `compilation_unit::` 35/63: 3 gains, 1 regression (corrected below)
  - `analyser::irules_event_checks` 0 regressions
  - `analyser::diagnostics::fp` 447 → 479 (+40) with 8 pass-to-fail:
    - 5 `fp_inj_*`: iRules fixtures with executable code at top level. Under the fresh rule load HTTP:: commands only exist in event bodies, and KCS IRULE5007 says the top level is declaration-only. I moved the negative fixtures into `when HTTP_REQUEST {…}` with assertions unchanged; this is documented in `fp/inj.rs` and accepted by the coordinator.
    - `fp_rbs_15_while1_break_in_opaque_switch_exits_loop`. With after-load, SCCP decides `while 1` as Always, and the lexical exit scan never looked inside a switch clause list. Fixed in `analyser/bounds_checks.rs::script_words`, which now recurses into case-list clause action bodies via `CaseListSpec` and `split_case_list`. Test: `bounds_checks::tests::w241_is_silent_when_a_switch_clause_breaks_the_loop` (tclsh 8.6/9.0: the loop ends).
    - `fp_sh_21_dict_literal_in_dict_for_silent`: still open.
    - `fp_tnt_03_interp_eval_list_literal_head_no_t105`: still open.
  - `compilation_unit::supplied_hosted_source_entry_retains_advice_without_activation` comes from the iRules fresh entry, not C6. I corrected it to the 2026-10-21T00:10Z decision: f5-irules is a RuleLoader entry with the NativeRecipe policy; a profile without the simulation keeps unknown history.
- **CRLF (coordinator's low-priority item).** `lowering/mod.rs::guarded_body_text` now maps a braced body through `segmenter::braced_body_source_region` before the `{body}x` clamp. Test: `lowering::tests::crlf_namespace_eval_body_unit_spans_its_written_region`.

## 6. Attribution answers (compiler-names' failures)

- **`signature_scan::walker::multiple_handlers_dispatch_correctly`: not mine.** Under the tcl8.6 fresh-interpreter entry, `source /abs/path.tcl` raises, so `proc bar` is unreachable; tclsh agrees.
- **The other 8 `signature_scan` failures** were already failing in my earlier-tree run and sit on Tcl paths I did not touch.
- **`source_graph::ordinary_rule_init_reporting_name_*`.** Probably predates me; I can't prove it without a baseline build. The hosted body's `set ::static::x` has no original variable operands, so `NormalTransferInvocation::mutation_places` returns `unknown_top`. That is the same mechanism as `irule4001_fires_for_static_write_outside_rule_init`, which was FAILED in my pre-iRules baseline. Still open.

## 7. Measurements (`tcl-compiler` lib filters, latest tree before C6, vs my earlier run `y3`)

| Filter | ok / failed | Pass→fail | Gains |
|---|---|---|---|
| `analyser::irules_event_checks` | 71 / 18 | 0 | 0 (net +1 vs y3 from the new declaration test) |
| `irules_checks::` | 59 / 57 | 0 | 4 |
| `analyser::diagnostics::tests` | 587 / 245 | 0 | 6 |
| `analyser::diagnostics::fp` | 447 / 134 | 0 | 2 |
| `registry_invocation` | 208 / 79 | 0 | 2 |
| `signature_scan::` | 191 / 7 | 0 | 2 |
| `taint::` | 45 / 17 | 0 | 2 |
| `source_graph::` | 1 / 2 | no baseline | both failures are §6 |
| `source_class::tests` | 6 pre-existing failures, unchanged | 0 | the new D tests pass |
| `depth_guard` new tests | pass (about 20 s each) | — | — |
