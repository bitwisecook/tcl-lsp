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

### tcl8.4

Status: `not-tested`. Version: not tested for this Rust contract. Build: not recorded. Channel: Rust source contract. Dialect: Tcl.

No native interpreter executes this current-point Rust proposal contract.

### tcl8.5

Status: `not-tested`. Version: not tested for this Rust contract. Build: not recorded. Channel: Rust source contract. Dialect: Tcl.

No native interpreter executes this current-point Rust proposal contract.

### tcl8.6

Status: `not-tested`. Version: not tested for this Rust contract. Build: not recorded. Channel: Rust source contract. Dialect: Tcl.

No native interpreter executes this current-point Rust proposal contract.

### tcl9.0

Status: `not-tested`. Version: not tested for this Rust contract. Build: not recorded. Channel: Rust source contract. Dialect: Tcl.

No native interpreter executes this current-point Rust proposal contract.

### tcl9.1

Status: `not-tested`. Version: not tested for this Rust contract. Build: not recorded. Channel: Rust source contract. Dialect: Tcl.

No native interpreter executes this current-point Rust proposal contract.

### jim

Status: `not-tested`. Version: not tested for this Rust contract. Build: not recorded. Channel: Rust source contract. Dialect: Jim Tcl.

No native interpreter executes this current-point Rust proposal contract.

### bigip

Status: `not-tested`. Version: not tested for this Rust contract. Build: not recorded. Channel: Rust source contract. Dialect: F5 iRules.

No native interpreter executes this current-point Rust proposal contract.

## Exact evidence

- `implementation` (implementation): [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs). SHA-256 `f03d37e07c84bfe360f0b3f9500f1eddc755ac5f4cb9cfe539d79d87487c4331`. Actual shared current-point issuer and fixed discriminating tests. The source digest records implementation correspondence and does not claim test execution.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs), `SourceInvocationBinding::original_scalar_assignment_command`: Retain current selected setter identity and original full source point separately from receiver/store completion.
- [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs), `OriginalScalarAssignmentCommand::matches_original_point`: Authenticate the unchanged original lookup snapshot, site and quiet selected command observers.
- [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs), `command_binding::insertion_template::tests::proposed_assignment_retains_current_registry_slot_and_source_point` (linked): All six authored engine contexts retain the current registered slot and original source point; foreign source, full grammar configuration and Registry selections reject.
- [rust/tcl-compiler/src/command_binding/insertion_template.rs](../../../../rust/tcl-compiler/src/command_binding/insertion_template.rs), `command_binding::insertion_template::tests::proposed_assignment_declines_missing_shadowed_wrapped_and_unknown_setters` (linked): Missing, unknown, source-procedure shadows and alias wrappers cannot supply a registered setter proposal.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked fixed Rust tests define the scoped implementation premises. Independent execution receipts belong in the validation ledger; this record supplies no native observation or Rust pass.
