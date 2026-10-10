# naming.variable.original-ssa-definition-operand

Kind: `implementation-contract`

## Problem statement

Opaque source name siblings can share reporting text. A normal-transfer capsule that reconstructs output addresses from those labels can attach an SSA definition or inferred type to another cell, even if the initial source producer retained the correct native units.

## Question

How does a selected SSA definition retain its exact original effective receiver operand and byte-cell identity?

## Conclusion

The source owner retains one full effective OriginalVariableInvocation at the actual handler boundary and withdraws conflicting observations. Normal definition and transfer consumers use its independent compiler/runtime operands and exact output ordinals. SsaSourceView matches an actual SSA definition cell and variable binding phase to the genuine written input; missing ownership never falls back to display text. The mapping supplies no value, future lifetime, Normal completion, representation, rename coverage or edit permission.

## Scope

Rust Compiler definition mapping and Core type hints under retained source image/configuration. Exact output origin, binding phase, current operand graph and cell identity are independent. Conditional source cards cannot fill a missing executed definition. Native provider behaviour is documented in separate empirical questions.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/original_variable_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/original_variable_inventory.rs), `OriginalVariableInvocations`: Retains once-issued effective operands at full original invocation sites with conflict withdrawal.
- [rust/tcl-compiler/src/variable_bindings.rs](../../../../rust/tcl-compiler/src/variable_bindings.rs), `OriginalVariableInvocation`: Keeps runtime input and independently selected compiler receiver purposes at exact effective ordinals.
- [rust/tcl-compiler/src/registry_invocation.rs](../../../../rust/tcl-compiler/src/registry_invocation.rs), `NormalTransferInvocation`: Projects normal handler definitions and variable effects through retained original operands.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `original_definition_name`: Matches an actual SSA cell/phase to the genuine original written input.
- [rust/tcl-compiler/src/place_bridge.rs](../../../../rust/tcl-compiler/src/place_bridge.rs), `def_places_at`: Selects original definition cells from genuine CFG point tokens even when specialised IR has no statement token vector; missing original ownership withdraws.
- [rust/tcl-compiler/src/command_binding/original_variable_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/original_variable_inventory.rs), `command_binding::original_variable_inventory::tests::original_operand_inventory_keeps_runtime_bytes_and_withdraws_missing_or_conflicting_owners` (linked): Retains two distinct opaque runtime operands and permanently withdraws missing or conflicting owners.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `ssa::original_definition_name_tests::original_ssa_definition_names_preserve_exact_operands_and_distinct_cells` (linked): Requires genuine point tokens for the first tokenless AssignConst and exact decoded original receiver inputs/distinct SSA byte cells for opaque siblings and an indexed target.
- [rust/tcl-lsp-core/src/inlay_hints.rs](../../../../rust/tcl-lsp-core/src/inlay_hints.rs), `inlay_hints::original_variable_type_tests::original_type_hints_match_exact_ssa_cells_without_reporting_variable_maps` (linked): Inferred type hints require the authentic SSA definition input and source declaration rather than reporting maps.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the exact linked Rust selectors through the maintained workspace test setup. This record contains no interpreter observation or executed Rust result.
