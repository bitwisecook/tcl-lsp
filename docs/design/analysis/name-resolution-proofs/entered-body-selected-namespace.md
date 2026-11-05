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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `EnteredScriptObservation`: Retain selected activation namespace separately from the exact executed source and frame.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `SourceCommandBindings::record_executed_script`: Capture entered namespace only alongside the actual entered frame.
- [rust/tcl-compiler/src/command_binding/conditional_body.rs](../../../../rust/tcl-compiler/src/command_binding/conditional_body.rs), `SourceCommandBindings::original_body_owns_context`: Compare independently retained entered frame and namespace against the genuine body owner.
- [rust/tcl-compiler/src/command_binding/conditional_body.rs](../../../../rust/tcl-compiler/src/command_binding/conditional_body.rs), `command_binding::conditional_body::original_body_frame_tests::entered_original_body_frames_keep_declaration_and_allocation_independent` (linked): Checks actual entered procedure ownership, declaration-frame independence, and rejection of a foreign namespace or different procedure allocation.

## Evidence

- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs)
- [rust/tcl-compiler/src/command_binding/conditional_body.rs](../../../../rust/tcl-compiler/src/command_binding/conditional_body.rs)
