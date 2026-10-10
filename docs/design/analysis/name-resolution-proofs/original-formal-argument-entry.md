# naming.compiler.original-formal-argument-entry

Kind: `implementation-contract`

## Problem statement

An unavailable current handler can erase a fresh formal's original incoming value before that handler is selected, even though its original argument entry and operand effects are independently retained.

## Question

Which declaration graph point can supply conditional incoming-formal provenance when current dispatch is unavailable, without claiming a physical read or entered handler?

## Conclusion

The same original declaration-flow graph retains incoming formals at the genuine pre-operand argument-entry site under its available-prefix paths. Joins and earlier unknown writers, stores and aliases withdraw the value. The formal producer separately validates whole source/configuration, declaration topology, fresh scalar receiver, local observer/alias state and all original operand effects before transporting entry provenance to the formal fetch. The current handler's unavailable selection cannot erase its own earlier argument entry. This does not select that handler or grant an actual read, caller value, CPP, Normal or execution.

## Scope

Conditional source-only direct formal provenance in the immutable declared frame. Actual body entry and runtime reads remain independent.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native observation or executed Rust result is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native observation or executed Rust result is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native observation or executed Rust result is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native observation or executed Rust result is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native observation or executed Rust result is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native observation or executed Rust result is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native observation or executed Rust result is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/declaration_flow.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_flow.rs), `DeclarationFlowReport::conditional_argument_keeps_incoming`: Return only incoming names retained on original available-prefix argument-entry paths.
- [rust/tcl-compiler/src/command_binding/declaration_flow.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_flow.rs), `Builder::evaluate`: Record the original command entry separately from later selected handler metadata.
- [rust/tcl-compiler/src/command_binding/formal_value.rs](../../../../rust/tcl-compiler/src/command_binding/formal_value.rs), `SourceCommandBindings::original_direct_formal_value`: Require original topology, quiet local receiver and operand-effect closure before consuming the conditional entry facet.
- [rust/tcl-compiler/src/command_binding/formal_value.rs](../../../../rust/tcl-compiler/src/command_binding/formal_value.rs), `command_binding::formal_value::tests::unentered_formal_advice_uses_original_snapshot_without_actual_read_or_cpp` (linked): Multiple opaque procedure declarations and calls preserve conditional incoming scalar advice with executable read/CPP/Normal receipts cleared. Prior unknown handlers, writes, nested operand writes, traces, aliases and caller links refuse the advice.

A named test is a coverage binding, not a claim that it executed.

## Replay

No Rust execution receipt is attached to this implementation contract. Tests are authored bindings only; physical interpreter observations remain separate.
