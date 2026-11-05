# naming.editor.logical-written-component-geometry

Kind: `implementation-contract`

## Problem statement

Logical namespace edits and source selection need exact written component positions. Collapsing a separator run loses byte geometry, and recovering namespace versus empty terminal entity meaning from a label loses the actual occurrence purpose. A failed component mapping cannot establish complete edit coverage.

## Question

Which exact original byte extents and selected occurrence purposes does the shared written-component owner retain for Logical namespace selection and edits, and when must component correspondence refuse?

## Conclusion

The shared Rust owner defines nonempty original byte extents, consuming each complete run of at least two colons and retaining lone colons, Unicode and NUL bytes. Logical consumers separately retain Namespace, Entity or configured Variable purpose, so trailing separators and actual braced headers keep their own meaning. Exact written suffix correspondence yields an original extent or an independently established inherited omission; failed correspondence remains terminal and never counts as occurrence coverage. Complete current source, metadata, grammar and edit guards remain required. One named syntax extent control passed in the separately retained software receipt; the other three finite controls have no attached assertion result. All seven external provider executions are not tested for this question.

## Scope

Current Rust source/API definitions in tcl-syntax and tcl-lsp-core, actual parent module registrations, and four linked software control definitions, one with a separately retained syntax-only pass. Raw byte segmentation is separate from configured variable header selection and positively selected Logical source edit geometry. The direct mapper refuses escaped/computed geometry without an authentic container. Whole original input/configuration/metadata and coverage/hazard checks remain independently required. Authored C/Jim/F5 profiles in model controls are not original provider processes. No Native lookup, namespace lifetime, current command/cell identity, header/cache, body entry, source-to-runtime byte equivalence, applied external edit or measured performance outcome is inferred.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Current Rust source/API and finite model definitions.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl8.4 execution or provider result is attached.

### tcl8.5

Status: `not-tested`. Channel: Current Rust source/API and finite model definitions.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl8.5 execution or provider result is attached.

### tcl8.6

Status: `not-tested`. Channel: Current Rust source/API and finite model definitions.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl8.6 execution or provider result is attached.

### tcl9.0

Status: `not-tested`. Channel: Current Rust source/API and finite model definitions.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl9.0 execution or provider result is attached.

### tcl9.1

Status: `not-tested`. Channel: Current Rust source/API and finite model definitions.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl9.1 execution or provider result is attached.

### jim

Status: `not-tested`. Channel: Current Rust source/API and finite model definitions.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original jim execution or provider result is attached.

### bigip

Status: `not-tested`. Channel: Current Rust source/API and finite model definitions.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original bigip execution or provider result is attached.

## Consumer bindings

- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `qualifier_component_extents`: Define exact nonempty original byte extents using complete runs of at least two colons; a lone colon remains a component byte. No trailing empty entity purpose, constructed-key inversion or source container is issued.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `qualifier_segments`: Collect the same shared extents directly as original byte slices; retain the separately selected trailing-separator purpose at consumers.
- [rust/tcl-syntax/src/naming/authored_namespace_component.rs](../../../../rust/tcl-syntax/src/naming/authored_namespace_component.rs), `AuthoredNamespaceOccurrencePurpose`: Retain independently issued Logical Namespace, Entity or Variable grammar purpose; a display label cannot supply this role.
- [rust/tcl-syntax/src/naming/authored_namespace_component.rs](../../../../rust/tcl-syntax/src/naming/authored_namespace_component.rs), `authored_namespace_component_extent`: Check the selected resolved suffix against the complete original written geometry and selected variable header grammar. Return exact Written, independently established Inherited, or terminal failed correspondence.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `lexical_namespace_rename_edits`: Require positively selected Logical advice and complete current source/metadata/configuration. Failed component mapping never counts as covered occurrence; existing coverage/hazard and source edit guards remain independent.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `original_namespace_rename_span_at_offset`: Retain the independent Native original operand, source container, current component path and checked original literal extent route. Logical geometry grants no authority to this separate route.
- [rust/tcl-lsp-core/src/rename.rs](../../../../rust/tcl-lsp-core/src/rename.rs), `invocation_replacement`: Use shared original written tail geometry for the existing source replacement worker; preserve complete separator runs independently of source edit permission.
- [rust/tcl-lsp-core/src/rename.rs](../../../../rust/tcl-lsp-core/src/rename.rs), `build_var_ref_replacement`: Retain actual configured variable header and array extent while projecting only the original written qualifier/tail; no runtime cell or source edit grant.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `written_separator_extent`: Project the existing shared complete written qualifier extent without parsing another namespace or converting to a constructed key.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `naming::tests::written_component_extents_share_segmentation_and_preserve_original_byte_geometry` (linked): The finite byte-vector control compares the shared component extents with qualifier_segments, including complete separator runs, lone colons, Unicode and NUL bytes. These are software definitions, not external interpreter or editable-container observations. This named syntax control passed in the separately retained software receipt; no Core edit or provider result follows.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `namespace_rename::tests::logical_written_namespace_geometry_retains_complete_separator_runs_and_purpose` (linked): Direct Logical mapping keeps namespace/entity/variable purposes distinct, complete separator widths and original Unicode positions. It separates inherited omission from failed correspondence and refuses changed written names and escaped direct geometry.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `namespace_rename::tests::logical_braced_variable_geometry_uses_the_actual_header_span_and_close_rule` (linked): The genuine analyser-produced braced Var span ends before its closer. The selected original braced-variable grammar supplies contained geometry, retaining parentheses and nested versus first-close meaning; malformed or mismatched headers refuse.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `namespace_rename::tests::logical_namespace_rename_maps_source_and_refuses_failed_occurrence_coverage` (linked): The retained Logical analyser/source model defines exact selected replacement extents and keeps failed mappings outside occurrence coverage. Changed source, configuration, occurrence projection and missing original input refuse. Authored C/Jim/F5 model profiles are not external provider executions.
- [rust/tcl-lsp-core/src/rename.rs](../../../../rust/tcl-lsp-core/src/rename.rs), `rename::tests::logical_rename_preserves_original_separator_runs_unicode_and_array_extent` (linked): Fixed source replacement helpers preserve complete separator runs, Unicode byte geometry and whole array suffixes under the supplied variable grammar. Actual Analyser source replacement retains current-image correspondence and declines changed source. These source extents require independent caller edit permission. Source definition only; no matched owned assertion outcome or new external-provider observation is attached.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `namespace_rename::tests::logical_namespace_forget_rename_retains_complete_source_and_hazard_guards` (linked): A genuine Logical Analyser namespace-forget producer returns exact Unicode replacement extents under complete current source and lexical admission. Changed input and deferred callback hazards refuse; the shared constructed qualification helper preserves literal-colon holders and the root marker. Source definition only; no matched owned assertion outcome or new external-provider observation is attached.

## Evidence

- [rust/tcl-lsp-core/src/lib.rs](../../../../rust/tcl-lsp-core/src/lib.rs)
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs)
- [rust/tcl-syntax/src/lib.rs](../../../../rust/tcl-syntax/src/lib.rs)
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs)
- [rust/tcl-lsp-core/src/rename.rs](../../../../rust/tcl-lsp-core/src/rename.rs)
