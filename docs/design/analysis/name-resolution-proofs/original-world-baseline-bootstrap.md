# naming.command.original-world-baseline-bootstrap

Kind: `implementation-contract`

## Problem statement

A first command can fork into an entered body and an unchanged branch before any command publication occurs. If only the entered operation selects the source world policy, joining it with an uninitialised baseline withdraws the world even though both branches share the same authentic source baseline. Reinitialising a later unavailable world would instead hide real unknown history.

## Question

Is the original command world selected from the immutable known source baseline before branching, while incompatible, native and unknown entry worlds retain their separate guards?

## Conclusion

The initial known source state selects its naming policy and independently catalogued namespace geometry once. Subsequent operations and branches retain that full structural baseline. Join still withdraws incompatible or unavailable worlds, and policy selection never revives unavailable history. No command installation, normal completion or native namespace incarnation follows from baseline selection.

## Scope

Implementation contract for independently selected source-only baseline policies before the first operation or branch. Supplied native entries and unknown entry history use their own state owners. Namespace deletion, mutation, original publication and complete root normal transfer remain independent. No native provider result is asserted.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not applicable. Build: not applicable. Channel: Rust implementation contract. Dialect: Tcl.

No native observation is asserted; the Rust contract has independent exact test selectors.

### tcl8.5

Status: `not-tested`. Version: not applicable. Build: not applicable. Channel: Rust implementation contract. Dialect: Tcl.

No native observation is asserted; the Rust contract has independent exact test selectors.

### tcl8.6

Status: `not-tested`. Version: not applicable. Build: not applicable. Channel: Rust implementation contract. Dialect: Tcl.

No native observation is asserted; the Rust contract has independent exact test selectors.

### tcl9.0

Status: `not-tested`. Version: not applicable. Build: not applicable. Channel: Rust implementation contract. Dialect: Tcl.

No native observation is asserted; the Rust contract has independent exact test selectors.

### tcl9.1

Status: `not-tested`. Version: not applicable. Build: not applicable. Channel: Rust implementation contract. Dialect: Tcl.

No native observation is asserted; the Rust contract has independent exact test selectors.

### jim

Status: `not-tested`. Version: not applicable. Build: not applicable. Channel: Rust implementation contract. Dialect: Jim Tcl.

No native observation is asserted; the Rust contract has independent exact test selectors.

### bigip

Status: `not-tested`. Version: not applicable. Build: not applicable. Channel: Rust implementation contract. Dialect: F5 iRules.

No native observation is asserted; the Rust contract has independent exact test selectors.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `OriginalSourceCommandWorld::for_baseline`: Once-only source baseline selection before any branch or command operation.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `ModuleCommandBindings::initial_from_baseline`: Build the original world alongside the same immutable canonical table baseline.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `command_binding::source_command_world::tests::original_world_baseline_is_selected_before_branching` (linked): Initial and unchanged selected branches share an equal policy/world; joining them stays available, while unavailable and unknown history cannot be reseeded.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact Rust selectors listed above exercise the implementation premises. No execution receipt or native provider observation is attached to this implementation contract.
