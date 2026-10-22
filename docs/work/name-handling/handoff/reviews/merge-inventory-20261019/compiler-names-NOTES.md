# compiler-names hand-back (2026-10-21/22)

Agent: compiler-names, shared `work` tree. All changes are in `rust/tcl-compiler`.
C Tcl (tclsh 8.5–9.1) decided every semantic question; the probes are quoted in the
tests.

## Root cause shared by the nested-name items (heads in `[..]`, C math functions)

Command substitutions are walked by `SourceCommandBindings::walk_substitutions` with
`source_purpose = NestedScript`. `record_declaration_operand_layout`
(`command_binding/declaration_layout.rs`) refused every non-`WholeInput` command in the
root branch. Nested commands therefore had no declaration-layout row at their own site.
Without that row they got no `original_name_input`, no `original_lookup` and no
`original_math_functions_in_source` row.

**Fix:** the root branch now also admits a `NestedScript` command when
`context.body_entry == Selected` and
`origin.original_lexical_command(words, offset, config)` proves that the vector is the
genuine original one at that offset. Conditional and future bodies are still refused,
and so is materialised text: a different origin, or words that do not match.

## Changes, by item

| Item | File(s) | Change |
|---|---|---|
| Nested heads (`puts [greet]`, `set a [greet]`, `[list [greet]]`, `"<[greet]>"`) | `command_binding/declaration_layout.rs` | Nested layout admission, as above |
| C: math functions in `[expr {...}]` | same root | Nothing more was needed |
| Consumed names (`info body greet`) | `command_binding/source_name_lookup.rs`, `analyser/scope.rs` | New `SourceInvocationBinding::original_consumed_command_lookup`. The operand must be a later written word of the consuming vector in every row. `finalise_original_invocation_inputs` gives `ConsumedName` rows that lookup. `retain_positioned_command_definitions` issues their reference with `point.command_reference(name)`, at the consuming site. `PossibleConsumedName` and `DeferredReference` are unchanged and still fail closed |
| `puts [info body greet]` (no row at all) | `analyser/commands.rs` | `dispatch_nested_segment` now calls `record_command_name_invocations` for substituted commands |
| R2: body `return` loses its lookup when called 2+ times | `command_binding/source_name_lookup.rs` | The binding joined from several call sites has `variable_context.namespace_known == false`. `original_home_scope` (Authored `::`) now also accepts the declaration row's own snapshot context |
| R1: completed command world withheld for `puts hi`, `set a [list ..]`, `[string length x]` | `command_binding.rs` (released to me) | New `SourceOutcomes::completed_normal_world`, used at both root sites. The world is the root normal-path state when every abrupt route is `Tcl(Error)` or `Unknown` (an uncatalogued handler residue beside a possible normal return), and the normal state is not opaque, step-observed or execution-observed, and not `unclosed_command_world`. The old proof is still accepted. `unclosed_command_world` is a new monotone `ModuleCommandBindings` history flag (joined like the opacity flags; in PartialEq, same_state, Hash and the observation stamp). It is set on a handler's normal state when the handler has no closed receipt and `command_resolution_impact()` of its declared transitions is not `None`, i.e. a binding, namespace-resolution, package-load or unknown transition. It is set in `walk_prepared_source_target` and on its early non-body result path. `exit` (ProcessExit), return/break/tailcall routes, unknown commands (opaque) and unclosed mutations still withhold the world. Env trace: `TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE` |
| B: `my`/`next` self-dispatch tape in nested bodies | `command_binding/source_transition_advice/body_effects.rs` | The member-body catalogue walk did not descend into `switch` with a computed subject: its authored roles were incomplete, so the command was not immediate and no regions were walked. New `member_catalogue_possible_bodies` follows the possible case arms and the `PotentialEvaluation` bodies whose own operand timing is `SameInvocation`. Each runs on its own branch with the parent uncertainty recorded, and nothing is merged back. It is skipped for deferred, procedure, definition, frame-shifting and other-interpreter forms (`proc nested {} {my ping}` is still refused) |
| `walker::multiple_handlers_dispatch_correctly` | `signature_scan/walker.rs` (test fixture) | Not a reachability problem: `proc bar {} {}` on its own also registered nothing. The test set a tcl8.6 registry but kept `LexerConfig::default()` (Tcl 9.0 escapes). `SignatureSourceNameKey::from_original_native_word` correctly refuses a word whose escape grammar differs from the 8.6 policy. The fixture now derives config and rules from the profile, as `extract_signatures` does. Assertions are unchanged |
| `variable_name::original_logical_roots_keep_exact_document_geometry_and_native_refusal` | `signature_scan/variable_name.rs` (test) | Coordinator ruling. tclsh 8.6.18, 9.0.4 and 9.1.0 scan `$::ns::café($index)` as `::ns::caf`: é ends the name (`can't read "::ns::caf": variable is array`). The docs give no Logical exception: name-handling.md:387 and shared-written-name-boundaries.md:102 describe an ASCII `$` scanner. The expectation is now `::ns::caf`, not an array root |

## Regression tests added

| Test | Status |
|---|---|
| `analyser::class_hierarchy::original_metadata::source_method_reference::tests::readonly_self_selectors_follow_nested_potential_evaluation_bodies` (12 nested forms × tcl8.6/9.0) | Pass |
| `command_binding::original_math_function::tests::original_math_occurrences_cover_expressions_in_command_substitutions` (tcl8.5–9.1) | Pass |
| `analyser::types::original_declaration_inventory_tests::substituted_heads_and_consumed_names_retain_original_lookup` | Pass |
| `analyser::types::original_declaration_inventory_tests::completed_world_follows_the_normal_completion_path_and_entered_bodies_keep_lookups` | Pass |
| `analyser::types::original_declaration_inventory_tests::crlf_class_method_bodies_analyse_without_lexer_panics` | **Fails**: open bug C11, see below |
| `analyser::diagnostics::tests::w210_pkgindex_reports_other_reads_inside_command_substitutions` | **Fails**: open bug C9, see below |

## Before/after (tcl-compiler `--lib`)

- **`command_binding::`**:
  - After the B/C/nested-lookup build: 861 ok / 316 FAILED, a FAILED set identical to sol45 `after-cmdbind`, so zero pass→fail.
  - After the first R1 cut (opaque checks only): 862 / 319, with 10 PASS→FAIL. All 10 assert that no completed world exists after an unclosed command-world mutation: `original_alias_creation…`, `original_command_mutation…`, `original_namespace_deletion…`, `original_namespace_binding::…import_completion…`, `original_class_factory_transfer…`, `deferred_method…overlapping_mixin…`, `source_class_publications…`, `source_procedure_publications…`, and `trusted_package_transfer…` ×2.
  - The `unclosed_command_world` refinement targets exactly those 10. **Its verification build was blocked by disk (<1.3 GB free at hand-back).** Rerun `command_binding::` and `completed_world_follows…` before relying on it.
- **`registry_invocation::`**: 208 / 79 FAILED. None of the 79 was ok in sol45 `after-reginv`, so there are zero pass→fail.
- **`source_graph::`**: 2 FAILED (`event_function_owner_tests::ordinary_rule_init_*`, Irule4001/6001). These are iRules hosted-body tests and belong to the compiler agent's iRules queue. No code path of this diff touches them.
- **`signature_scan::`**: 9 FAILED before my fixture/expectation fixes; 2 of them now pass. The other 7 are work-only tests, all absent on origin/rust:
  - `ctx::original_hosted_source_metadata_tests::*nested_bom_purpose`
  - `factory::tests::*` (×2)
  - `formal_parameters::tests::*` (×2)
  - `tests::signature_publication_keys_match_six_native_namespace_controls`
  - `original_name::original_import_publication_tests::*`

  None of them involves substitutions, consumed names or the member catalogue. `signature_publication_keys…` and the factory tests are likely the same class of fixture/config mismatch as `multiple_handlers`, but this is not yet confirmed.
- **`analyser::`**: the full filter aborts the test process partway (a stack overflow; see the luna-41 C10 lowering recursion). I could not obtain a complete before/after; the earlier partial run had no FAILED attributable to this diff.

## Open findings handed back (not fixed here)

1. **C9, W210 in pkgIndex.** Not caused by the nested-layout change: the results are identical with that admission disabled. In the real analyser lowering at tcl8.6, every `set f [<builtin> ...]` gets a leading Barrier `"embedded call runs an unreadable script at the global frame"`. Every `set`/`source` gets a trailing CallerFrameOpaque barrier `"<cmd> upvar-aliases a dynamic caller variable"`, and `Call.reads` is empty. `[file join $dir $missing]` reports only the first undefined read. This sits in cfg_builder/lowering (`upvar_effect_statements` / `embedded_subst_extras`).
2. **Import alias.** In `namespace eval ::b {namespace import ::a::p; p}` (tcl8.6), and in the tcl9.0 `Factory` case, `apply_import` does not fail and `p` does not resolve as an unknown binding. The declaration row's `original_command_world.scope(Authored("::b"))` is still None, so `original_home_scope` returns None and there is no lookup or reference. The top-level `namespace import ::a::p; p` works. Next step: `source_command_world.rs` namespace registration across the import transition (the compiler agent's file).
3. **C11, CRLF.** Repro: `Analyser::analyse("oo::class create Widget {\r\n    method render {} {\r\n        puts \"widget $label\"\r\n    }\r\n}", "tcl8.6")` panics with the `scan_var_ref` debug assertion. Call path: `var_refs::original_variable_reference_form` ← `record_arg_var_reads` ← `analyse_body` ← `walk_method_body` ← `parse_oo_definition_body`. The computed offset is +3, which equals the number of CRs before `$label`. A direct `segment_commands_with_offset_and_config` of the same source keeps the CRs and its spans agree. The CR loss is therefore in the analyser's own class-definition path (`call.args` / member texts), not the segmenter.
4. **next/nextto LSP tests.** The compiler side is correct: `source_registry_words` gives the `if` Body span, `next` has `TCLOO_NEXT_CHAIN`, and `nextto Base` has a Name role with literal `Base`. The remaining failure is in tcl-lsp-core (`references.rs` `method_next_dispatch_spans` gating).
5. **R3**, the class `variable n` unification, is not started.
