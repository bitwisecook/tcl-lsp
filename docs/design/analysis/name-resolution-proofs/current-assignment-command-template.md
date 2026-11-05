# naming.compiler.current-assignment-command-template

Kind: `implementation-contract`

## Problem statement

An extraction can introduce a scalar assignment at an existing source operation. A fresh destination variable does not establish which command an inserted setter spelling would invoke: the current namespace may shadow, wrap, delete or observe the registered setter. Using the completed module world or rebuilding reached tokens for the proposed command would also lose the original insertion point. This check applies to authored command proposals at a retained reached source point; it does not establish a successful variable store.

## Question

Does the scalar-assignment command proposal retain the actual current registered setter slot and original point, and decline missing, unknown, shadowed or wrapped selections?

## Conclusion

The shared issuer selects the registered Set operation through the retained original point and canonical first-occupied command paths. It requires the selected unwrapped builtin identity, current Registry and naming policy, full source configuration and quiet command observers before rendering a callable source word. Missing, unknown, foreign, shadowed and alias-wrapped selections decline. Receiver availability, store completion, value release, edit placement and native preparation remain independent. This is a Rust implementation contract; native interpreters do not execute it.

## Scope

Authored scalar setter proposals under the five selected Tcl authoring contexts and Jim, with genuine original source origin, full parser configuration, point-current command occupancy and Registry identity. No fabricated original operand, native command object, opcode, handler admission, receiver availability or Normal completion is issued.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`. Channel: Rust source contract. Version: tcl8.4 not tested for this Rust contract; tcl8.5 not tested for this Rust contract; tcl8.6 not tested for this Rust contract; tcl9.0 not tested for this Rust contract; tcl9.1 not tested for this Rust contract; jim not tested for this Rust contract; bigip not tested for this Rust contract.

No native interpreter executes this current-point Rust proposal contract.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs), `SourceInvocationBinding::original_scalar_assignment_command`: Retain current selected setter identity and original full source point separately from receiver/store completion.
- [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs), `OriginalScalarAssignmentCommand::matches_original_point`: Authenticate the unchanged original lookup snapshot, site and quiet selected command observers.
- [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs), `command_binding::insertion_template::tests::proposed_assignment_retains_current_registry_slot_and_source_point` (linked): All six authored engine contexts retain the current registered slot and original source point; foreign source, full grammar configuration and Registry selections reject.
- [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs), `command_binding::insertion_template::tests::proposed_assignment_declines_missing_shadowed_wrapped_and_unknown_setters` (linked): Missing, unknown, source-procedure shadows and alias wrappers cannot supply a registered setter proposal.

## Evidence

- [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs)
