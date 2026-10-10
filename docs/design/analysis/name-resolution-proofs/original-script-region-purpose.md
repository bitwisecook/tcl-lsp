# naming.source.original-script-region-purpose

Kind: `implementation-contract`

## Problem statement

Reference-only script syntax is useful to navigation but does not describe a possible evaluation. An unqualified shared body inventory can accidentally lend those regions to executable or mutation consumers.

## Question

How does one shared original script geometry inventory retain reference-only syntax while exposing only potential-evaluation regions to executable and mutation consumers?

## Conclusion

OriginalSourceScriptPurpose selects Syntax or PotentialEvaluation over the same complete original Registry words, source/configuration/context and body/case geometry. Syntax retains reference-only body text. PotentialEvaluation uses the already selected script timing at each effective original ordinal and excludes ReferenceOnly; it remains conditional and proves no reached body, runtime frame, effects closure or Normal. Mutation coverage uses the same purpose projection. The default source_script_bodies method retains its Syntax contract; Core executable-region traversal explicitly asks for PotentialEvaluation. Hosted event source paths select PotentialEvaluation through the same shared body-purpose owner; their ReferenceOnly regions remain independently retained syntax without event reachability or appliance activation.

## Scope

One shared geometry owner and selected Registry timing owner. Genuine reference-only and potential Body descriptors are distinguished without a new parser or reporting-head lookup. Unsupported source geometry/context stays unavailable. Source syntax and potential evaluation do not authorise runtime execution or edits.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/selected_script_timing.rs](../../../../rust/tcl-registry/src/selected_script_timing.rs), `SelectedScriptMetadata::timing_at`: Select timing from the same actual role/options/prefix layout at the effective original ordinal.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `ResolvedInvocation::authored_source_script_timing_at`: Expose already-selected source timing without recapturing a nominal command or replacing the full context.
- [rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs), `OriginalSourceScriptPurpose`: Keep syntax and potential-evaluation purposes explicit, independent of runtime body entry.
- [rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs), `source_script_bodies_for`: Project the same retained original body/case geometry according to the requested purpose.
- [rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs), `source_script_bodies_for_mutation_coverage`: Exclude reference-only slots from the potential mutation inventory through the shared purpose.
- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `OriginalRegistryWords::source_script_bodies_for`: Retain sealed vector/source/context correspondence in the common adapter.
- [rust/tcl-lsp-core/src/executable_regions.rs](../../../../rust/tcl-lsp-core/src/executable_regions.rs), `ExecutableWalker::original_source_regions`: Require PotentialEvaluation in the actual-analysis executable traversal.
- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs), `OriginalRegistryWords::source_script_bodies_for`: Delegate source body projection and its explicit Syntax/PotentialEvaluation purpose through the retained Compiler descriptor.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `original_interp_visibility_body`: Choose PotentialEvaluation for actual conditional child-script source visibility; reference-only source text cannot supply a child body.
- [rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs), `barrier_body_locally_sets`: Require original potential-evaluation body applicability for lexical store suppression; a reference-only Body role alone is insufficient.
- [rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs), `registry_invocation::source_scoped_body::script_purpose_tests::original_script_inventory_separates_reference_syntax_from_potential_evaluation` (linked): An actual custom ContextRegistry reference-only Body remains in Syntax but not PotentialEvaluation or mutation coverage. The ordinary Body keeps its original region and selected SameInvocation timing. No body executes.
- [rust/tcl-lsp-core/src/executable_regions.rs](../../../../rust/tcl-lsp-core/src/executable_regions.rs), `executable_regions::tests::retained_analysis_regions_respect_reference_only_script_purpose` (linked): A custom retained C8.6 source context retains ReferenceOnly body Syntax geometry but no executable region; the ordinary Body retains potential-evaluation traversal without executing it.
- [rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs), `analyser::diagnostics::dataflow::source_body_purpose_tests::original_reference_only_body_cannot_supply_local_store_suppression` (linked): Actual Logical selected Body versus ReferenceOnly roles retain distinct conditional suppression purposes. Original script geometry does not establish executed store or entered child.
- [rust/tcl-irules/src/source_context.rs](../../../../rust/tcl-irules/src/source_context.rs), `source_context::tests::original_irules_source_paths_keep_reference_only_bodies_as_syntax` (linked): Custom genuine hosted Body source remains visible as Syntax under ReferenceOnly, while the event-rooted conditional path requires PotentialEvaluation. Complete source/context remains the same and neither branch proves Native event or body execution.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
