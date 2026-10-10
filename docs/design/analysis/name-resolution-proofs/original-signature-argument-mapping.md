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

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

This is a Rust source-consumer contract. No executed interpreter or appliance observation is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

This is a Rust source-consumer contract. No executed interpreter or appliance observation is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

This is a Rust source-consumer contract. No executed interpreter or appliance observation is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

This is a Rust source-consumer contract. No executed interpreter or appliance observation is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

This is a Rust source-consumer contract. No executed interpreter or appliance observation is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust source-consumer contract. No executed interpreter or appliance observation is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust source-consumer contract. No executed interpreter or appliance observation is attached.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs). SHA-256 `f3313fc883d47ef4291af7ee4e4517bdb1852c1aeae0561d4aab19a34c68790c`. Current Rust owner for this readonly source purpose; this digest is not an execution receipt.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs). SHA-256 `bf33691691a26d096d80dc82cab309df12979f7c2ffaa735663db775d4cfd93e`. Current Rust owner for this readonly source purpose; this digest is not an execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs), `OriginalRegistryWords::active_argument_at`: Select one effective Registry argument from its own written or expanded source origin; captured values supply no cursor extent.
- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs), `effective_argument_at`: Check procedure argument cardinality, frozen original values and exact expanded-child extents before returning a cursor ordinal.
- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs), `selected_registry_words`: Retain unanimous effective command advice, exact frozen operand values and independent original producers under current complete source/configuration.
- [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs), `signature_help`: Project the checked original call vector into a readonly selected parameter using the retained Registry and source profile.
- [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs), `signature_help::tests::original_signature_argument_mapping_keeps_captured_prefixes_without_source_anchors` (linked): Captured builtin/procedure prefixes shift the active parameter without creating written source positions.
- [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs), `signature_help::tests::original_signature_expansion_cursor_selects_its_own_list_child` (linked): Each original expansion child selects its own effective parameter for Registry and procedure advice.
- [rust/tcl-lsp-core/src/signature_help.rs](../../../../rust/tcl-lsp-core/src/signature_help.rs), `signature_help::tests::original_signature_currency_and_registry_survive_reporting_counterfactuals` (linked): Reporting-map/dialect changes cannot replace the retained Registry; changed full source or grammar declines advice.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors are linked coverage obligations. Their execution outcome is separate; no native provider replay or physical runtime authority is claimed.
