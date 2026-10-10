# naming.compiler.original-variable-alias-source-schema

Kind: `implementation-contract`

## Problem statement

Alias declaration completion needs the original local operand and its own naming frame. A reported target name or an unrelated complete read cannot establish the local declaration, an entered link or the currently selected target cell.

## Question

How can original global and upvar declaration receipts expose readonly local source candidates without requiring a complete alias read or granting target-cell authority?

## Conclusion

Original declaration assistance borrows only exact NUL-free UTF8 control words through shared source_schema_word while preserving counted inputs, original ordinals and source geometry. Registry-selected alias purpose, frame and genuine target correspondence independently issue the private declaration receipt. AnalysisResult::original_variable_alias_advice_in_source validates the whole original image and full configuration and returns borrowed OriginalVariableAliasAdvice for the receipt’s own local input, declaration span and frame. Both global and upvar local candidates remain available without a complete read. Core variable completion selects only advice in the authentic cursor frame and keeps source spelling independent of reporting maps. A complete read retains its own occurrence matching; declaration advice supplies no entered link, selected access, current target cell, value, editable reference coverage or execution permission. Known shadowed handlers, dynamic selectors, expansion and stale source decline these declaration candidates.

## Scope

Readonly original alias declaration schema and current source completion under authentic selected frame/target/purpose receipts. Counted or opaque operand inputs are retained independently of the narrow text control projection. Source declaration visibility does not establish link success, runtime activation, Normal, physical storage or rename authority.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs](../../../../rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs), `original_declaration_assistance`: Select unanimous original alias purpose and written operand ordinals from the actual Registry grammar, using shared exact source control projection.
- [rust/tcl-compiler/src/analyser/types.rs](../../../../rust/tcl-compiler/src/analyser/types.rs), `AnalysisResult::original_variable_alias_advice_in_source`: Borrow current whole-image/full-config declaration receipts without requiring a complete alias read or granting current target contents.
- [rust/tcl-compiler/src/signature_scan/variable_symbol.rs](../../../../rust/tcl-compiler/src/signature_scan/variable_symbol.rs), `OriginalVariableAliasAdvice`: Expose only the genuine declaration local input, original frame, local naming bytes and source span; target-cell and entered-link receipts remain private independent axes.
- [rust/tcl-lsp-core/src/completion/original_variables.rs](../../../../rust/tcl-lsp-core/src/completion/original_variables.rs), `items`: Select current cursor-frame alias declaration source spellings independently of reporting maps and complete read occurrences.
- [rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs](../../../../rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs), `registry_invocation::declaration_assistance::original_alias_source_schema_tests::original_alias_source_schema_keeps_counted_controls_and_written_operands` (linked): Authentic global/shared and upvar/link declaration candidates are retained independently of the complete link read; stale source, custom shadow handlers, dynamic level and expanded names decline source advice.
- [rust/tcl-lsp-core/src/completion/original_variables.rs](../../../../rust/tcl-lsp-core/src/completion/original_variables.rs), `completion::original_variables::tests::original_variable_completion_selects_actual_formal_frame_and_alias_spelling` (linked): After reporting variable maps and frame labels are erased, genuine cursor-frame completion retains original formals, local declarations and both global/upvar local spellings, including aliases without a complete read.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact Rust selectors bind readonly source declaration assertions. No Rust execution receipt or native provider observation is attached to this implementation contract.
