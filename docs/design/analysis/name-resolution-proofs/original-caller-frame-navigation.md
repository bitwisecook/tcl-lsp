# naming.core.original-caller-frame-navigation

Kind: `implementation-contract`

## Problem statement

A fresh compatibility Realm or reporting scope can replace the actual source input, namespace or caller frame and cross a nested frame.

## Question

How do caller-variable navigation candidates retain the full actual source input, configuration and namespace through typed caller templates without granting upvar stores or entered cells?

## Conclusion

The shared caller-frame projection consumes the complete retained analysis input and genuine SourceStructure command/body words. Typed Compiler caller templates select same-frame source reference extents; namespace, whole source, full configuration and ContextRegistry generation remain current. NavigationVariable accepts a declaration-backed original symbol or a sealed caller-frame source template. Orphan roots and display names do not become candidates. Original-only rename selection remains separate. The integrated caller-frame reference component uses that same actual retained Registry/input and genuine read/call geometry. A foreign resolution Registry, changed full source or absent input cannot rebuild a nominal caller scan.

## Scope

Rust readonly navigation advice only. No successful upvar store, actual frame entry, physical cell, contents, native operand, Normal or editable reference coverage is granted.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust source-selection invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust source-selection invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust source-selection invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust source-selection invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust source-selection invariant.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No interpreter observation establishes this Rust source-selection invariant.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No interpreter observation establishes this Rust source-selection invariant.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/caller_frame.rs](../../../../rust/tcl-lsp-core/src/caller_frame.rs), `caller_frame_bindings`: Retain actual source input/configuration/namespace when selecting typed caller-frame source templates.
- [rust/tcl-lsp-core/src/caller_frame.rs](../../../../rust/tcl-lsp-core/src/caller_frame.rs), `original_caller_read_spans`: Consume already selected caller bindings and preserve genuine same-frame variable-root extents.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `NavigationVariable`: Keep declaration-backed symbols and sealed caller-frame source candidates separate from original-only rename selection.
- [rust/tcl-compiler/src/command_binding/conditional_body.rs](../../../../rust/tcl-compiler/src/command_binding/conditional_body.rs), `SourceCallerFrameInvocationTemplate`: Retain the original Registry frame layout and complete original body correspondence without successful binding or frame entry.
- [rust/tcl-lsp-core/src/caller_frame.rs](../../../../rust/tcl-lsp-core/src/caller_frame.rs), `caller_frame::tests::original_caller_templates_keep_full_input_bom_namespace_and_source_currency` (linked): Five C authoring inputs retain LeadingBom Skip, actual namespace N, same-input clones and authentic scopes after reporting maps clear; changed BOM/source or equivalent foreign full-input context allocation refuse.
- [rust/tcl-lsp-core/src/caller_frame.rs](../../../../rust/tcl-lsp-core/src/caller_frame.rs), `caller_frame::tests::original_caller_reference_geometry_does_not_cross_a_nested_frame` (linked): One C9 conditional source retains its genuine call and two same-frame variable-root extents while an unrelated nested procedure root is excluded.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `variable_symbol::tests::original_navigation_keeps_caller_templates_separate_from_unbound_symbols` (linked): A C9 map-erased caller template retains one declaration/two references and one independent readonly card without a declaration; changed whole source and upvar 0/#0/2 controls refuse this template.
- [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs), `references::tests::caller_frame_reference_component_keeps_retained_registry_and_input` (linked): Genuine current custom Registry and original caller read/call words retain two source references; foreign Registry, missing input and changed image refuse. No executed variable link or current cell is inferred.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact Rust selectors are linked coverage. No native interpreter observation establishes this implementation ownership contract.
