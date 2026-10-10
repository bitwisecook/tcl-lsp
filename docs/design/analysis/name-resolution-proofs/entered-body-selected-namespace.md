# naming.source.entered-body-selected-namespace

Kind: `implementation-contract`

## Problem statement

A procedure or lambda activation can retain a namespace selected independently of its frame wrapper. Reading only the wrapper loses an authored namespace identity and can confuse equal displayed names with different allocations.

## Question

How does an entered body retain the namespace selected by its genuine activation while remaining distinct from its original declaration frame?

## Conclusion

EnteredScriptObservation retains the exact executed source, the actual entered frame and the independently selected namespace from that activation. Deferred observations have neither entered frame nor namespace. Body ownership compares the exact source, allocation, frame and selected namespace; another body or namespace cannot supply the receipt.

## Scope

Rust entered-source and conditional declaration ownership. The namespace is captured from the same selected procedure, receiver or lambda entry and passed to the existing common source driver. It is never derived from a reporting name or a generated object spelling. No body entry, Normal completion or native scope is manufactured by this projection.

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

- `implementation-0` (implementation): [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs). SHA-256 `97be1d16e74f5a6dd6a8f834c55727e38e353b56358c7a23d6884887b6dcbf6e`. Reviewed Rust owner for this implementation invariant; this file is not an executed provider capture.
- `implementation-1` (implementation): [rust/tcl-compiler/src/command_binding/conditional_body.rs](../../../../rust/tcl-compiler/src/command_binding/conditional_body.rs). SHA-256 `e3bed9bb852fa4d5bdb31ad5a7829b92d2105100535435de6dc4508fac7c1b8c`. Reviewed Rust owner for this implementation invariant; this file is not an executed provider capture.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `EnteredScriptObservation`: Retain selected activation namespace separately from the exact executed source and frame.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `SourceCommandBindings::record_executed_script`: Capture entered namespace only alongside the actual entered frame.
- [rust/tcl-compiler/src/command_binding/conditional_body.rs](../../../../rust/tcl-compiler/src/command_binding/conditional_body.rs), `SourceCommandBindings::original_body_owns_context`: Compare independently retained entered frame and namespace against the genuine body owner.
- [rust/tcl-compiler/src/command_binding/conditional_body.rs](../../../../rust/tcl-compiler/src/command_binding/conditional_body.rs), `command_binding::conditional_body::original_body_frame_tests::entered_original_body_frames_keep_declaration_and_allocation_independent` (linked): Checks actual entered procedure ownership, declaration-frame independence, and rejection of a foreign namespace or different procedure allocation.

A named test is a coverage binding, not a claim that it executed.

## Replay

No interpreter observation is attached. Rust execution and original native namespace recipes remain separately recorded.
