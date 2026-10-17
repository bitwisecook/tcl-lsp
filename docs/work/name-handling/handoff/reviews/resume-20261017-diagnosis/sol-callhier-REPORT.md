# Sol call_hierarchy/code_lens diagnosis (orchestrator-recorded copy of the hand-back)

44 failing selectors at HEAD 2a3928bc6; 42 pre-existing since capture21, 2 regressions (admission26 ca7bbf745 property census; admissions22-24 ab88e9f0d objdefine inline).

- G1 property census never completes (regression). Defects: source_structure.rs:1582-1607 census_command_bodies global head lookup inside definition bodies; original_declaration.rs:1018-1025 member_matches compares against all_tokens incl. Sep. fix-1. Also admission26 control readonly_property_reports_promote_only_a_complete_typed_source_census fails at :1757; its :1823 foreign resolved_input assertion separate, uninvestigated. Callback-retaining commands (after 0 [list my size]) are a fail-closed hole guarded in fix-3.
- G2 inline oo::objdefine member (regression): handlers.rs:6352-6360 requires authored_source_definition_body_grammar (None without script operand). fix-2.
- G3 method lenses hard-coded Partial (original_declaration.rs:1121-1146). G3a fix-3 (after fix-1, medium-low). G3b 8 typed external-receiver tests need design. G3c lens_unknown_instance_method_is_not_counted, lens_bare_word_in_method_body_is_not_a_self_dispatch need absence-as-completeness (constraint conflict); lens_counts_obj_method_call_inside_proc_body receiver untyped through global.
- G4 native call hierarchy lacks `my` edges: original_call_hierarchy.rs:487-583. fix-4.
- G5 edge after unknown command: source_procedure_edges ignores uncertain_operations/has_lookup_operand_effects. fix-5.
- G6 missing compiler producers: method->proc, my callbacks, classmethod Factory make, same-named method/classmethod. outgoing_calls_from_proc_reach_bare_class_dispatch expects "::build" vs others "caller" (label conflict).
- G7 tests conflicting with native semantics (tclsh8.6 confirmed): same_named_procs_in_different_namespaces_disambiguate_by_span (unknown namespace), classmethod absent under tcl8.6 (2 tests), incoming_calls_for_target_* string-label API empty in native by design, itcl without package require (2), consumed_command_names_remain_references_without_becoming_calls (PossibleConsumedName vs ConsumedName).
- G8 Logical path misses calls in deferred bodies: logical_procedure_invocation.rs:178-243 (4 tests, low confidence).
- G9 original_own_object_calls_and_references_keep_the_canonical_object_worker second cause at prepare :926.
