# naming.server.original-reference-and-lens-source-selection

Kind: `implementation-contract`

## Problem statement

Cross-document references and reference lenses can disagree when one consumer selects original byte declarations and another reparses a displayed qualified name. Equal display text, namespace siblings and copied provider documents can then create unrelated locations or hide ambiguity.

## Question

How do workspace reference, declaration and reference-lens consumers select the same independently current original source owner without reparsing reporting names?

## Conclusion

The shared server selector prepares a sealed readonly declaration identity from the complete participating source inventory and genuine original caller lookup. Reference and lens locations reuse that identity through the canonical source reference kernel. Declaration inclusion is explicit, and each returned span is lifted against its independent URI and rechecked current source after asynchronous reads. Duplicate providers, missing original selection and stale source decline the whole advice path; Native misses do not enter reporting-name resolution. This selects source declarations and occurrences, not native dispatch.

## Scope

Rust server workspace source-selection and location transport. Fixed C8.6 source-analysis controls preserve opaque sibling keys, namespace priority, independent provider URI ownership, inclusion policy and duplicate-provider refusal. Missing operands and counterfactual report entries remain terminal. The linked tests compare the shared reference helper and cross-document declaration path; they do not execute an interpreter, assert native object state, or certify a wire handle from client labels.

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

- `implementation-0` (implementation): [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs). SHA-256 `2cb6ca20a493d45b8a2b8105fe59c8454867e337feda537cf026bb0f64bd9293`. Current Rust owner for this readonly source purpose; this digest is not an execution receipt.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs). SHA-256 `cd2fe6dd100dc3e6f74d1838dd6b9ac6415dea1da7a34f1e258dda5805acf9a1`. Current Rust owner for this readonly source purpose; this digest is not an execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_identity_at`: Select one current document-owned readonly identity through genuine original prepare and complete participating source inventory.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_reference_locations`: Use the canonical reference kernel for references and lenses, keep declaration inclusion explicit, and recheck each URI/source before projecting UTF-16 spans.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `reference_locations`: Route original namespace, variable, method and procedure/class queries through their typed source owners before compatibility resolution.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `references`: Retain actual original lookups and canonical allocation/declaration owners for source occurrence enumeration; source advice grants no native dispatch.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_workspace_references_and_lenses_share_owned_byte_declarations` (linked): The shared source/reference path selects pD800 under its provider URI, excludes pD801 and namespace siblings, respects include-declaration and declines duplicate authentic providers.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_workspace_reference_misses_never_reparse_reporting_names` (linked): Counterfactual reporting entries cannot repair a missing original query; changed source cannot reissue an existing provider identity.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors are linked coverage obligations. Their execution outcome is separate; no native provider replay or physical runtime authority is claimed.
