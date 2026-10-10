# naming.variable.original-alias-source-template

Kind: `implementation-contract`

## Problem statement

A complete original upvar operand can name a fixed array root while its substituted index stays unknown. Physical alias matching correctly lacks a selected element cell, but readonly definitions, hover and references still need to share the genuine written local-to-root relationship without promoting that relationship into an executed alias or an editable reference set.

## Question

Which authentic source inputs, frame correspondence and source-order barriers can support conditional navigation from a local alias declaration and later local roots to a fixed target root while leaving array index, target cell, successful alias and binding continuity unproved?

## Conclusion

AnalysisResult::original_variable_alias_templates_in_source requires the complete current image, full retained resolved input/configuration and genuine original declaration frame. Registry-selected alias purpose uses the shared original_variable_alias_local_name projection. The sealed template retains its own local input and authentic target producer; a compound target keeps its unknown evaluated index separate. Nearest preceding direct source declarations and exact newer alias operands determine conditional source relationships; dynamic later targets and intervening conditional declarations block stale later anchors. Same-frame original read/write roots supply their own geometry. Core NavigationVariable uses one separate Alias arm for readonly definitions, hover and references. Existing actual-cell matching and original-only rename selection are unchanged. Typed VariableCellAlias original subject/local argument ordinals are projected through the retained actual OriginalRegistryWords source schema and its complete ContextRegistry. Only Written effective origins with a genuine whole NativeWord contribute source spans; captured prefix and expanded element origins cannot borrow spans. The sealed source operand facet remains retained separately through per-item merging and clearing. The native alias receipt still independently requires an original declaration frame, local input and matching target input policy. Unknown target values, array indices and cells are not selected. Original source alias operands consume the selected Registry successful-layout transition recipe conditionally. The ordinary pre-dispatch transition remains unknown for computed upvar targets in Tcl 8.4/8.5; source pair operands can still describe the written relationship if that handler completes successfully. No successful edge, target cell, installed alias, Native allocation or mutation continuity is established. The genuine whole source schema and Written operand origins remain necessary; computed target element indices stay unknown.

## Scope

Authored Rust controls in C8.4, C8.5, C8.6, C9.0 and C9.1 source contexts. No new native interpreter result or Rust execution receipt is inferred. The template always carries WrittenAliasApplicability, UnavailableTargetCell and UnprovedBindingContinuity. It issues no current alias, entered activation, evaluated index, selected element, store completion, Normal, Native compilation or refactoring coverage. Jim and BIG-IP are not measured by these selectors.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-advice invariant.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/signature_scan/variable_symbol.rs](../../../../rust/tcl-compiler/src/signature_scan/variable_symbol.rs), `OriginalVariableAliasTemplate`: Separate conditional source relationship and unresolved obligations from the physical original alias receipt; share purpose-specific local-key geometry with declaration receipt construction.
- [rust/tcl-compiler/src/signature_scan/variable_symbol.rs](../../../../rust/tcl-compiler/src/signature_scan/variable_symbol.rs), `original_variable_alias_local_name`: Separate conditional source relationship and unresolved obligations from the physical original alias receipt; share purpose-specific local-key geometry with declaration receipt construction.
- [rust/tcl-compiler/src/analyser/variable_alias_templates.rs](../../../../rust/tcl-compiler/src/analyser/variable_alias_templates.rs), `AnalysisResult::original_variable_alias_templates_in_source`: Authenticate complete retained source/input/configuration, original frames, direct declaration sites, nearest source-order receipts and newer alias barriers.
- [rust/tcl-compiler/src/command_binding/declaration_layout.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_layout.rs), `SourceCommandBindings::original_variable_alias_template_matches_at_span`: Match original local producer and genuine declaration frame/byte-primary geometry without a Place or CellIdentity; physical alias matcher stays separate.
- [rust/tcl-compiler/src/analyser/scope.rs](../../../../rust/tcl-compiler/src/analyser/scope.rs), `Analyser::original_variable_alias_receipt`: Use the same original-variable-alias local projection for genuine declaration receipts.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `NavigationVariable`: Project one conditional alias source relationship into readonly definitions, explicit conditional hover and exact reference extents; original-only rename selection remains separate.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `select_navigation`: Project one conditional alias source relationship into readonly definitions, explicit conditional hover and exact reference extents; original-only rename selection remains separate.
- [rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs](../../../../rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs), `OriginalSourceVariableAliasOperands`: Retain the distinct authentic alias source operand facet through whole-source schema selection and per-item currency. Typed source ordinals do not issue target values, entered links or cells.
- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `original_registry_words_for_tokens`: Retain the distinct authentic alias source operand facet through whole-source schema selection and per-item currency. Typed source ordinals do not issue target values, entered links or cells.
- [rust/tcl-compiler/src/analyser/commands.rs](../../../../rust/tcl-compiler/src/analyser/commands.rs), `record_original_variable_receivers`: Retain the distinct authentic alias source operand facet through whole-source schema selection and per-item currency. Typed source ordinals do not issue target values, entered links or cells.
- [rust/tcl-compiler/src/analyser/scope.rs](../../../../rust/tcl-compiler/src/analyser/scope.rs), `original_variable_alias_receipt`: Retain the distinct authentic alias source operand facet through whole-source schema selection and per-item currency. Typed source ordinals do not issue target values, entered links or cells.
- [rust/tcl-compiler/src/analyser/per_item.rs](../../../../rust/tcl-compiler/src/analyser/per_item.rs), `clear_fragment_original_producers`: Retain the distinct authentic alias source operand facet through whole-source schema selection and per-item currency. Typed source ordinals do not issue target values, entered links or cells.
- [rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs](../../../../rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs), `OriginalSourceVariableAliasOperands::capture`: Original source alias operands consume the selected Registry successful-layout transition recipe conditionally. The ordinary pre-dispatch transition remains unknown for computed upvar targets in Tcl 8.4/8.5; source pair operands can still describe the written relationship if that handler completes successfully. No successful edge, target cell, installed alias, Native allocation or mutation continuity is established. The genuine whole source schema and Written operand origins remain necessary; computed target element indices stay unknown.
- [rust/tcl-compiler/src/analyser/variable_alias_templates.rs](../../../../rust/tcl-compiler/src/analyser/variable_alias_templates.rs), `analyser::variable_alias_templates::tests::original_alias_source_templates_keep_unknown_elements_and_source_order_separate` (linked): Five C source contexts retain exactly the own alias declaration, later lappend word and first lexical read in one original frame; fixed target root n/a retains neither complete whole-word key nor produced value and has UnavailableTargetCell. Sibling read is absent. Stale image, changed only LeadingBom and copied foreign Jim resolved input refuse. A later dynamic alias and a nested conditional alias block stale following anchors; a newer direct alias selects its own target root b. Typed VariableCellAlias source ordinals retain only Written effective origins and whole NativeWord spans under actual full schema/context; captured and expanded producers cannot borrow source anchors.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `variable_symbol::tests::original_alias_navigation_joins_name_templates_without_original_cell_selection` (linked): Five C source contexts with String reporting maps erased select a distinct Alias navigation arm from its own declaration, later write and lexical read; original target declaration, exact four source anchors and explicit conditional/unproved hover agree. Original-only variable selection does not become target-cell selection. Sibling read and copied foreign Jim input refuse the Alias arm. Typed VariableCellAlias source ordinals retain only Written effective origins and whole NativeWord spans under actual full schema/context; captured and expanded producers cannot borrow source anchors.
- [rust/tcl-lsp-core/src/caller_frame.rs](../../../../rust/tcl-lsp-core/src/caller_frame.rs), `caller_frame::caller_frame_navigation_tests::a_fully_qualified_upvar_target_navigates_from_word_and_read` (linked): The fully-qualified array target and local alias source anchors retain exact definition and hover correspondence. References include exactly (2,10), (2,34), (3,12), (7,21) and (8,18), including the genuine info-exists root. These are conditional source relationships, not a concrete array element or successful upvar link.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked Rust selectors are current source-relationship coverage obligations. No native process or executed Rust result is inferred; current physical alias and editable-reference authority remain separate.
