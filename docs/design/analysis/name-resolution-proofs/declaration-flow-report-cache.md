# naming.source.declaration-flow-report-cache

Kind: `implementation-contract`

## Problem statement

Repeated diagnostic queries rebuild the same declaration flow from immutable original layouts and variable observations. Reusing a report by displayed source or a partial entry would mix distinct source channels, frame owners or Registry generations.

## Question

When can diagnostic consumers share a declaration-flow report while keeping its complete original inventory and query identity?

## Conclusion

A builder retains one immutable inventory only while all five layout, access and point axes compare equal. Reports are shared as Arc values under the full original diagnostic entry, full LexerConfig, full InvocationDialect and RegistrySemanticKey. The report cache retains at most 64 entries and is excluded from structural equality and hashing. Builder clones begin with an empty derived cache; changed inventories replace it. Report construction runs outside the cache lock.

## Scope

Rust declaration-flow query and cache contract. Full original source bytes/channel, selected entry and namespace remain part of the retained entry. Hashes and Arc addresses are not semantic lookup identities. Cache reuse establishes neither native compiler admission nor Normal completion. Performance improvement is not an observation in this record.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-compiler/src/command_binding/declaration_flow.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_flow.rs). SHA-256 `1c59b8cad710ddb11dc3c18bf85c143ab03638a037093018bf09fce77a09c16a`. Reviewed Rust owner for this implementation invariant; this file is not an executed provider capture.
- `implementation-1` (implementation): [rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs). SHA-256 `23a6715e7cde3633a4db0f6f30dfeb131771310f37667cecbc3c62380017e123`. Reviewed Rust owner for this implementation invariant; this file is not an executed provider capture.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/declaration_flow.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_flow.rs), `DeclarationFlowInventory`: Compare complete immutable layout, variable-access and binding-point inputs while excluding derived report state.
- [rust/tcl-compiler/src/command_binding/declaration_flow.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_flow.rs), `DeclarationFlowKey`: Retain the complete original entry, full configuration and dialect, and Registry semantic identity for each query.
- [rust/tcl-compiler/src/command_binding/declaration_flow.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_flow.rs), `SourceInvocationBinding::declaration_flow_report`: Build outside the mutex and return the bounded shared report for an equal complete query.
- [rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs), `function_declaration_flow`: Preserve the shared immutable report through diagnostic consumers.
- [rust/tcl-compiler/src/command_binding/declaration_flow.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_flow.rs), `command_binding::declaration_flow::tests::original_declaration_flow_cache_retains_full_inventory_and_query_identity` (linked): Checks repeated report reuse, full-config and Registry changes, equal displayed bytes under a different source channel/entry, isolated cloned builders, inventory mutation, and held old-report stability.

A named test is a coverage binding, not a claim that it executed.

## Replay

No timing result or Rust execution is asserted here. Exact frozen-source builds and targeted performance receipts are retained separately in the Rust validation ledger.
