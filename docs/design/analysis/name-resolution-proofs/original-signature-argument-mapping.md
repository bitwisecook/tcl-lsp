# naming.consumer.original-signature-argument-mapping

Kind: `implementation-contract`

## Problem statement

A captured alias prefix shifts effective argument positions without writing those values at the cursor. A single expanded list word can supply several independent effective operands. Mapping either through display arguments selects the wrong signature parameter.

## Question

How does signature help select the effective argument belonging to the actual original cursor while retaining captured prefixes, expanded-child values and complete source configuration?

## Conclusion

Registry and procedure advice retain the actual effective vector and each original operand origin. Captured prefix values occupy argument slots without acquiring source anchors. An expanded child needs its own retained extent and exact value equality; ambiguous or unavailable geometry declines the cursor mapping. Current complete source, full grammar and retained Registry/profile are checked independently of reporting dialect labels. The resulting parameter ordinal and signature remain readonly advice.

## Scope

Rust signature-help and effective-argument source correspondence. Fixed C8.4–C9.1 analysis controls cover captured builtin/procedure prefixes; C8.5–C9.1 controls cover independent list expansion children; current-source/configuration and counterfactual reporting labels are separate negatives. No interpreter is launched by these selectors. No native alias object, callback execution, command availability, runtime dispatch or edit permission is supplied.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust source-consumer contract. No executed interpreter or appliance observation is attached.

## Consumer bindings

- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs), `OriginalRegistryWords::active_argument_at`: Select one effective Registry argument from its own written or expanded source origin; captured values supply no cursor extent.
- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs), `effective_argument_at`: Check procedure argument cardinality, frozen original values and exact expanded-child extents before returning a cursor ordinal.
- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs), `selected_registry_words`: Retain unanimous effective command advice, exact frozen operand values and independent original producers under current complete source/configuration.
- [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs), `signature_help`: Project the checked original call vector into a readonly selected parameter using the retained Registry and source profile.
- [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs), `signature_help::tests::original_signature_argument_mapping_keeps_captured_prefixes_without_source_anchors` (linked): Captured builtin/procedure prefixes shift the active parameter without creating written source positions.
- [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs), `signature_help::tests::original_signature_expansion_cursor_selects_its_own_list_child` (linked): Each original expansion child selects its own effective parameter for Registry and procedure advice.
- [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs), `signature_help::tests::original_signature_currency_and_registry_survive_reporting_counterfactuals` (linked): Reporting-map/dialect changes cannot replace the retained Registry; changed full source or grammar declines advice.

## Evidence

- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs)
- [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs)
