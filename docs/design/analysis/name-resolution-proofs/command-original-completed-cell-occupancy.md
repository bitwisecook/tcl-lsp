# naming.command.original-completed-cell-occupancy

Kind: `implementation-contract`

## Problem statement

A native source edit can target an occupied Registry or imported command even when no source declaration card exists. Inspecting source publications alone loses that collision; converting a displayed name into a new native key loses the original naming policy and namespace identity.

## Question

Does completed source-world collision advice query the same exact canonical command cell, including baseline bindings, without treating unknown holders or alternate bindings as a fresh destination?

## Conclusion

The completed-world facade requires its retained policy and known exact namespace holder, then reads the canonical binding alternatives for that byte slot. A definitely missing cell reports false; unanimously occupied source, imported or baseline cells report true. Unknown holders, incompatible policies, empty or mixed alternatives decline. No native insertion, edit, implementation or new completion receipt is issued.

## Scope

Read-only Rust advice from an independently complete Normal root source world, exact ByteCommandSlot and independently retained NamePolicyProtocol. Source publication, current baseline binding and imported wrapper occupancy remain independent of callable target selection. Unknown source prefixes and missing namespace holders do not issue absence; actual guest execution and native editing are excluded. The selector is a validation obligation without an attached passing execution receipt.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No guest execution of this Rust implementation question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No guest execution of this Rust implementation question is claimed.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `OriginalCompletedCommandWorld::command_slot_occupied`: Read-only exact canonical cell occupancy under the already issued complete Normal world, retaining unknown and mixed alternatives.
- [rust/tcl-compiler/src/command_binding/original_command_table.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_table.rs), `ModuleCommandBindings::original_bindings_for_key`: The same actual source/imported/baseline table owner; Registry metadata cannot fill supplied Native entry absence.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `command_binding::source_command_world::tests::original_completed_occupancy_keeps_baseline_source_and_unknown_holders_separate` (linked): Closed empty worlds retain stock baseline occupancy; missing cells, missing holders, policy mismatch and opaque moved source names remain separate, while unknown completion has no world.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `command_binding::source_command_world::tests::original_opaque_procedure_completion_and_body_keep_distinct_allocations` (linked): Actual closed source interpretation preserves two opaque procedure publication slots and the precise call allocation across selected C models; invalid original formal topology has no completed root world.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact Rust selectors exercise the scoped implementation premises. No Rust execution receipt or native provider observation is attached. No passing Rust or guest execution receipt is attached. The complete root world, exact holder and policy, known table alternatives and separate editing/lookup authority remain required.
