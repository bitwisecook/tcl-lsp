# naming.analysis.procedure-summary-candidate-keys

Kind: `implementation-contract`

## Problem statement

Display labels can collide across original declarations. Procedure-effect summaries must retain each issued body identity and its genuine original Module, allocation, source body and signature; missing ownership cannot select a body or erase possible caller effects.

## Question

How do shared original procedure owners retain distinct body-keyed summaries and signatures, select conditional original call edges, and preserve protective by-name reads without treating a display label as Native lookup or effect-absence authority?

## Conclusion

OriginalProcedureSummaryOwners retains the actual unchanged Module and its original Registry correspondence. Module.original_source_procedure_bodies enumerates primary procedures with the same Module issuer; OriginalDeclaration body units additionally require their actual declaration site, allocation, source image and body kind. Module.original_source_procedure_for_target uniquely joins a genuine target allocation/site and authored implementation image. Complete retained Module correspondence independently validates current signatures before OriginalProcedureSummaryOwners accepts its owner. CFG caller-frame, parameter and outer-write summaries remain keyed by those identities; equal labels do not merge distinct signatures or alias targets. Missing, foreign or changed ownership, a missing body, stale original vectors and unmatched retained metadata decline source selection or retain opaque caller hazards. Original execution and Logical source purposes may supply conditional MAY call summaries with their original effective arguments; Native source_procedure_for_target and Native callable admission remain independent. Protective ProcIndex candidates preserve every colliding parameter-summary possibility and union literal by-name arguments/defaults. Missing or dynamic keys protect possible literal names without proving an effect-free callee. Shared constructed key_suffixes remains pure presentation geometry for that protective index. Elimination retains its independent binding/opaque CFG, positioned original reads and RHS/raise/store proof owners. Linked fixed controls describe distinct body summaries, original signatures, changed/foreign ownership refusal and protective unions. One named constructed-label syntax control passed in the separately retained software receipt; no other current assertion, external provider or Native acceptance is attached.

## Scope

Current original-source procedure inventory, issued body-keyed CFG frame/parameter/global summaries and protective constructed-key ProcIndex projections. Conditional source summaries supply MAY effects, not Native command admission, namespace incarnation, physical frame/cell contents, successful completion or store-removal authority. Registry-only template memo identity remains independently certified; source-bound raw memo identity retains its actual Module owner. C Tcl, Jim and BIG-IP behaviour and whole optimisation conformance require their independent original oracle and consumer controls.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs. Version: tcl8.4 No external provider execution is attached to this constructed summary-index implementation question.

External tcl8.4 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### tcl8.5

Status: `not-tested`. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs. Version: tcl8.5 No external provider execution is attached to this constructed summary-index implementation question.

External tcl8.5 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### tcl8.6

Status: `not-tested`. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs. Version: tcl8.6 No external provider execution is attached to this constructed summary-index implementation question.

External tcl8.6 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### tcl9.0

Status: `not-tested`. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs. Version: tcl9.0 No external provider execution is attached to this constructed summary-index implementation question.

External tcl9.0 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### tcl9.1

Status: `not-tested`. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs. Version: tcl9.1 No external provider execution is attached to this constructed summary-index implementation question.

External tcl9.1 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### jim

Status: `not-tested`. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs. Version: jim No external provider execution is attached to this constructed summary-index implementation question.

External jim execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### bigip

Status: `not-tested`. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs. Version: bigip No external provider execution is attached to this constructed summary-index implementation question.

External bigip execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

## Consumer bindings

- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `key_suffixes`: One pure possible-summary-key projection over existing constructed key_segments; no written/native inverse or lookup selection.
- [rust/tcl-compiler/src/cfg_builder/summary_candidates.rs](../../../../rust/tcl-compiler/src/cfg_builder/summary_candidates.rs), `OriginalProcedureSummaryOwners`: Retain the genuine complete Module/Registry owner; original issued body candidates and effective arguments select conditional MAY summaries, never display-name lookup or Native callable admission.
- [rust/tcl-compiler/src/cfg_builder/mod.rs](../../../../rust/tcl-compiler/src/cfg_builder/mod.rs), `detect_upvar_bodies_with_registry`: Keep independent caller-frame summaries for each actual issued body and original signature; missing or changed source ownership and unavailable call edges retain opaque caller hazards.
- [rust/tcl-compiler/src/interprocedural.rs](../../../../rust/tcl-compiler/src/interprocedural.rs), `ProcParameterSummary`: Pure per-candidate parameter traits/defaults; no selected callee or absence evidence.
- [rust/tcl-compiler/src/interprocedural.rs](../../../../rust/tcl-compiler/src/interprocedural.rs), `build_proc_index_from_summaries`: Retain every colliding candidate and no first-winner HashMap policy.
- [rust/tcl-compiler/src/interprocedural.rs](../../../../rust/tcl-compiler/src/interprocedural.rs), `collect_call_by_name_reads`: Protective MAY name union only; exact neutral input keys or literal missing-target arguments, with actual binding/CFG/Native read owners remaining separate.
- [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs), `matches_original_procedure`: Existing full same-source procedure header/body correspondence; display keys cannot replace original retained declaration membership.
- [rust/tcl-compiler/src/optimiser/elimination.rs](../../../../rust/tcl-compiler/src/optimiser/elimination.rs), `dead_store_observation_unbounded`: Existing opaque Native access/CFG observer refusal remains before store advice; this projection never proves Native read absence.
- [rust/tcl-compiler/src/ir.rs](../../../../rust/tcl-compiler/src/ir.rs), `Module::original_source_procedure_bodies`: Enumerate primary procedures under the same issuing Module and OriginalDeclaration body units with their actual declaration site/allocation/image/body kind; retained signature correspondence is independently checked by the original Module owner.
- [rust/tcl-compiler/src/ir.rs](../../../../rust/tcl-compiler/src/ir.rs), `Module::original_source_procedure_for_target`: Uniquely join the genuine target allocation/site and authored implementation image to an issued body in the same Module; independent retained owner checks validate current signatures and Native source_procedure_for_target admission remains unchanged.
- [rust/tcl-compiler/src/cfg_builder/mod.rs](../../../../rust/tcl-compiler/src/cfg_builder/mod.rs), `prepare_cfg_context_bundle`: Retain the actual original Module owner alongside body-keyed parameter/frame/outer-write summaries and closed command bindings.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `naming::shared_boundary_tests::key_suffixes_preserve_constructed_colon_segments_without_lookup_authority` (linked): The named fixed syntax control checks constructed-label suffix candidates, including repeated colons, empty/trailing forms, Unicode and NUL, without lookup authority. It passed in the separately retained software receipt; it does not test procedure headers, frame summaries, Native behavior or provider execution.
- [rust/tcl-compiler/src/cfg_builder/mod.rs](../../../../rust/tcl-compiler/src/cfg_builder/mod.rs), `cfg_builder::tests::distinct_body_summaries_preserve_conflicting_alias_targets` (linked): Actual original namespace declarations retain distinct issued bodies and conflicting left/right alias targets; ambiguous forwarding retains no definite alias and opaque caller reads/writes. Definition only; current matched execution remains independent.
- [rust/tcl-compiler/src/cfg_builder/mod.rs](../../../../rust/tcl-compiler/src/cfg_builder/mod.rs), `cfg_builder::tests::summary_parameters_retain_each_original_signature` (linked): Each original issued body retains its own ordered parameters, defaults and variadic placement, including conflicting equal-tail declarations. Definition only; current matched execution remains independent.
- [rust/tcl-compiler/src/cfg_builder/mod.rs](../../../../rust/tcl-compiler/src/cfg_builder/mod.rs), `cfg_builder::tests::summary_keys_decline_one_changed_member_and_foreign_module_source` (linked): Changed original parameters withhold only the changed body parameter summary and retain its opaque frame hazard; a replaced foreign whole source withholds all parameter summaries and retains opaque frame effects. Original assertions unchanged; no current execution acceptance.
- `rust/tcl-compiler/tests/inlining_interproc_residual.rs`, `protective_proc_index_unions_every_colliding_parameter_and_default_read` (linked): Fixed source-defined constructed labels, complete retained procedure headers, original whole-module frame summaries and protective MAY read/default unions. No assertion outcome, selected Native dispatch, frame/cell/value, lookup absence or provider execution is attached.
- `rust/tcl-compiler/tests/inlining_interproc_residual.rs`, `protective_proc_index_missing_key_retains_literal_possible_name_arguments` (linked): Fixed source-defined constructed labels, complete retained procedure headers, original whole-module frame summaries and protective MAY read/default unions. No assertion outcome, selected Native dispatch, frame/cell/value, lookup absence or provider execution is attached.

## Evidence

- [rust/tcl-syntax/src](../../../../rust/tcl-syntax/src)
- [rust/tcl-compiler/src](../../../../rust/tcl-compiler/src)
- [rust/tcl-compiler](../../../../rust/tcl-compiler)
- [rust/tcl-compiler/tests](../../../../rust/tcl-compiler/tests)
- [rust/tcl-compiler/src/command_binding](../../../../rust/tcl-compiler/src/command_binding)
- [rust/tcl-compiler/src/optimiser](../../../../rust/tcl-compiler/src/optimiser)
- [rust/tcl-compiler/src/optimiser/elimination](../../../../rust/tcl-compiler/src/optimiser/elimination)
- [rust/tcl-compiler/src/registry_invocation](../../../../rust/tcl-compiler/src/registry_invocation)
