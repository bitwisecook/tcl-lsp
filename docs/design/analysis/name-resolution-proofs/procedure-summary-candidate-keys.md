# naming.analysis.procedure-summary-candidate-keys

Kind: `implementation-contract`

## Problem statement

Display labels can collide across original declarations. Procedure-effect summaries must retain each issued body identity and its genuine original Module, allocation, source body and signature; missing ownership cannot select a body or erase possible caller effects.

## Question

How do shared original procedure owners retain distinct body-keyed summaries and signatures, select conditional original call edges, and preserve protective by-name reads without treating a display label as Native lookup or effect-absence authority?

## Conclusion

OriginalProcedureSummaryOwners retains the actual unchanged Module and its original Registry correspondence. Module.original_source_procedure_bodies enumerates primary procedures with the same Module issuer; OriginalDeclaration body units additionally require their actual declaration site, allocation, source image and body kind. Module.original_source_procedure_for_target uniquely joins a genuine target allocation/site and authored implementation image. Complete retained Module correspondence independently validates current signatures before OriginalProcedureSummaryOwners accepts its owner. CFG caller-frame, parameter and outer-write summaries remain keyed by those identities; equal labels do not merge distinct signatures or alias targets. Missing, foreign or changed ownership, a missing body, stale original vectors and unmatched retained metadata decline source selection or retain opaque caller hazards. Original execution and Logical source purposes may supply conditional MAY call summaries with their original effective arguments; Native source_procedure_for_target and Native callable admission remain independent. Protective ProcIndex candidates preserve every colliding parameter-summary possibility and union literal by-name arguments/defaults. Missing or dynamic keys protect possible literal names without proving an effect-free callee. Shared constructed key_suffixes remains pure presentation geometry for that protective index. Elimination retains its independent binding/opaque CFG, positioned original reads and RHS/raise/store proof owners. Linked fixed controls describe distinct body summaries, original signatures, changed/foreign ownership refusal and protective unions. No current assertion, external provider or Native acceptance is attached.

## Scope

Current original-source procedure inventory, issued body-keyed CFG frame/parameter/global summaries and protective constructed-key ProcIndex projections. Conditional source summaries supply MAY effects, not Native command admission, namespace incarnation, physical frame/cell contents, successful completion or store-removal authority. Registry-only template memo identity remains independently certified; source-bound raw memo identity retains its actual Module owner. C Tcl, Jim and BIG-IP behaviour and whole optimisation conformance require their independent original oracle and consumer controls.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: No external provider execution is attached to this constructed summary-index implementation question.. Build: Fixed pure geometry and actual whole-source software producer/control definitions only.. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs.. Dialect: tcl8.4.

External tcl8.4 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### tcl8.5

Status: `not-tested`. Version: No external provider execution is attached to this constructed summary-index implementation question.. Build: Fixed pure geometry and actual whole-source software producer/control definitions only.. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs.. Dialect: tcl8.5.

External tcl8.5 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### tcl8.6

Status: `not-tested`. Version: No external provider execution is attached to this constructed summary-index implementation question.. Build: Fixed pure geometry and actual whole-source software producer/control definitions only.. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs.. Dialect: tcl8.6.

External tcl8.6 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### tcl9.0

Status: `not-tested`. Version: No external provider execution is attached to this constructed summary-index implementation question.. Build: Fixed pure geometry and actual whole-source software producer/control definitions only.. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs.. Dialect: tcl9.0.

External tcl9.0 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### tcl9.1

Status: `not-tested`. Version: No external provider execution is attached to this constructed summary-index implementation question.. Build: Fixed pure geometry and actual whole-source software producer/control definitions only.. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs.. Dialect: tcl9.1.

External tcl9.1 execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### jim

Status: `not-tested`. Version: No external provider execution is attached to this constructed summary-index implementation question.. Build: Fixed pure geometry and actual whole-source software producer/control definitions only.. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs.. Dialect: jim.

External jim execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

### bigip

Status: `not-tested`. Version: No external provider execution is attached to this constructed summary-index implementation question.. Build: Fixed pure geometry and actual whole-source software producer/control definitions only.. Channel: Source-only constructed metadata keys and retained original Module/procedure/CFG inputs.. Dialect: bigip.

External bigip execution is not tested or applicable as proof of this pure metadata/candidate policy. The source-defined controls prove no selected Native lookup, command token, frame, cell, contents, read absence or successful completion. Independently recorded native naming and BIG-IP TMM observations remain separately scoped and unchanged.

## Exact evidence

- `procedure-candidate-source-0` (implementation): [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs). SHA-256 `4808699cc117bb840c8ce9e9360bd7186f3f294ffb239d8ece2b75279ebf52f4`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-1` (implementation): [rust/tcl-compiler/src/cfg_builder/mod.rs](../../../../rust/tcl-compiler/src/cfg_builder/mod.rs). SHA-256 `c6b8c308542e8c132213d169ad7279c274e0bed2ef02db70324b9f9307173879`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-2` (implementation): [rust/tcl-compiler/src/cfg_builder/summary_candidates.rs](../../../../rust/tcl-compiler/src/cfg_builder/summary_candidates.rs). SHA-256 `3e634c0cd8ffcfac36055fc4b9601290e7a79a0ba5053854b9041b7f7d8e59f5`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-3` (implementation): [rust/tcl-compiler/src/interprocedural.rs](../../../../rust/tcl-compiler/src/interprocedural.rs). SHA-256 `d6ff4d9794342575e98837deaa39fe29ccf93b6f56e43a64b5a10f139b844139`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-4` (implementation): [rust/tcl-compiler/tests/inlining_interproc_residual.rs](../../../../rust/tcl-compiler/tests/inlining_interproc_residual.rs). SHA-256 `eb98e2920d8690e8da5d3b32d3e5364065a49ec1743a3dffc957fa9eeea38651`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-5` (implementation): [rust/tcl-compiler/src/ir.rs](../../../../rust/tcl-compiler/src/ir.rs). SHA-256 `141881e65e31d98be82d0b384d67a4e6a75da110503351d8c53d53df506c8b5b`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-6` (implementation): [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs). SHA-256 `914ceb69a4e2ab42af4b0ac99ede3a8c3374fc7743090488fe29b7cf0126e181`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-7` (implementation): [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs). SHA-256 `924e7447dc103d9a4b71a5a32a106228835dabaed48675b71fdf8e6b1aff49e2`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-8` (implementation): [rust/tcl-compiler/src/cfg_builder/upvar_info.rs](../../../../rust/tcl-compiler/src/cfg_builder/upvar_info.rs). SHA-256 `89f7fa436df00343d9a7654c2b8d54a293e4c83a34bfee8065fbc1396822b0db`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-9` (implementation): [rust/tcl-compiler/src/cfg_builder/global_write_info.rs](../../../../rust/tcl-compiler/src/cfg_builder/global_write_info.rs). SHA-256 `3b785d37161557605eed119feb43019951a61e30ff19c2d75c7447084c335367`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-10` (implementation): [rust/tcl-compiler/src/deferred_writes.rs](../../../../rust/tcl-compiler/src/deferred_writes.rs). SHA-256 `1a0af959696eab75bcfe528abaaa3fa92d73d8c3489b9c73d9f514368dcd3099`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-11` (implementation): [rust/tcl-compiler/src/optimiser/elimination.rs](../../../../rust/tcl-compiler/src/optimiser/elimination.rs). SHA-256 `1b0614f575a343275692126420b585ec4bc48b01c5c26355b2bc143266a99513`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-12` (implementation): [rust/tcl-compiler/src/optimiser/elimination/source_reads.rs](../../../../rust/tcl-compiler/src/optimiser/elimination/source_reads.rs). SHA-256 `c12544bf58a1db8794763844ca3f1d18ae37597e2448cb2bfbc44d190a87755e`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-13` (implementation): [rust/tcl-compiler/src/cfg.rs](../../../../rust/tcl-compiler/src/cfg.rs). SHA-256 `415526f876b3dc030502286c64a1cfc914a3d1d640a98eeb063de9b929bbc977`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-14` (implementation): [rust/tcl-compiler/src/registry_invocation/metadata_context.rs](../../../../rust/tcl-compiler/src/registry_invocation/metadata_context.rs). SHA-256 `d6585ad7853c73f600b5f7a1bc0ba2dd59728cce7d1d0b944ef029c14a066ffd`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-15` (implementation): [rust/tcl-compiler/src/lib.rs](../../../../rust/tcl-compiler/src/lib.rs). SHA-256 `00ee19bed242452aadbc7d038e91512a225ce7d5731a14718aa8c6b85dd06759`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.
- `procedure-candidate-source-16` (implementation): [rust/tcl-syntax/src/lib.rs](../../../../rust/tcl-syntax/src/lib.rs). SHA-256 `6ad16dac695833e45052276f0523986885877dc00634e10e06ff81e2d62d93e5`. Current canonical constructed geometry, original source-owner/CFG producer, protective metadata consumer or genuine fixed software control definition. Source-only; no execution outcome or Native authority.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

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
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `naming::shared_boundary_tests::key_suffixes_preserve_constructed_colon_segments_without_lookup_authority` (linked): Fixed source-defined constructed labels, complete retained procedure headers, original whole-module frame summaries and protective MAY read/default unions. No assertion outcome, selected Native dispatch, frame/cell/value, lookup absence or provider execution is attached.
- [rust/tcl-compiler/src/cfg_builder/mod.rs](../../../../rust/tcl-compiler/src/cfg_builder/mod.rs), `cfg_builder::tests::distinct_body_summaries_preserve_conflicting_alias_targets` (linked): Actual original namespace declarations retain distinct issued bodies and conflicting left/right alias targets; ambiguous forwarding retains no definite alias and opaque caller reads/writes. Definition only; current matched execution remains independent.
- [rust/tcl-compiler/src/cfg_builder/mod.rs](../../../../rust/tcl-compiler/src/cfg_builder/mod.rs), `cfg_builder::tests::summary_parameters_retain_each_original_signature` (linked): Each original issued body retains its own ordered parameters, defaults and variadic placement, including conflicting equal-tail declarations. Definition only; current matched execution remains independent.
- [rust/tcl-compiler/src/cfg_builder/mod.rs](../../../../rust/tcl-compiler/src/cfg_builder/mod.rs), `cfg_builder::tests::summary_keys_decline_one_changed_member_and_foreign_module_source` (linked): Changed original parameters withhold only the changed body parameter summary and retain its opaque frame hazard; a replaced foreign whole source withholds all parameter summaries and retains opaque frame effects. Original assertions unchanged; no current execution acceptance.
- [rust/tcl-compiler/tests/inlining_interproc_residual.rs](../../../../rust/tcl-compiler/tests/inlining_interproc_residual.rs), `protective_proc_index_unions_every_colliding_parameter_and_default_read` (linked): Fixed source-defined constructed labels, complete retained procedure headers, original whole-module frame summaries and protective MAY read/default unions. No assertion outcome, selected Native dispatch, frame/cell/value, lookup absence or provider execution is attached.
- [rust/tcl-compiler/tests/inlining_interproc_residual.rs](../../../../rust/tcl-compiler/tests/inlining_interproc_residual.rs), `protective_proc_index_missing_key_retains_literal_possible_name_arguments` (linked): Fixed source-defined constructed labels, complete retained procedure headers, original whole-module frame summaries and protective MAY read/default unions. No assertion outcome, selected Native dispatch, frame/cell/value, lookup absence or provider execution is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

Six source-defined fixed software controls permit later owned-executable reconfirmation. No assertion outcome or external C/Jim/BIG-IP process is attached; metadata geometry cannot certify Native namespace/command lookup.
