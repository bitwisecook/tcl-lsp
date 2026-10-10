# naming.variable.fresh-scalar-proposal

Kind: `implementation-contract`

## Problem statement

A refactor cannot infer a free runtime local from an absent reporting variable. Existing aliases, undefined allocated arrays, observers, retired slots or unknown incoming state can make a proposed assignment select another cell or run callbacks.

## Question

What independent facts allow a proposed new scalar name at a genuine original operation point?

## Conclusion

The reached proposal requires an actual entered Bound local activation with tracked complete local contents and independently closed activation observers. Exact native naming must preserve an unchanged unqualified scalar slot in that activation, with no occupied or previously retired slot, aliases, unknown bindings or read/write/unset observer residual. SsaSourceView retains the original reached invocation and full configuration separately. Namespace, global and event availability is not inferred, and the receipt grants no setter selection, new store, value, Normal completion, source movement or edit.

## Scope

Rust point-owned local absence contract. The fixed all-six model test distinguishes independently entered quiet locals from occupied slots, missing incoming inventory, alias uncertainty and new observer effects. Source-only conditional future entry and host/global availability need separate owners; no native provider observation is attached.

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

- [rust/tcl-compiler/src/var_resolve/fresh_scalar_proposal.rs](../../../../rust/tcl-compiler/src/var_resolve/fresh_scalar_proposal.rs), `fresh_scalar_variable_slot`: Requires actual complete local and observer inventories under exact selected native name purpose.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `fresh_scalar_variable_proposal`: Binds the absence receipt to a genuine reached original source operation/configuration.
- [rust/tcl-compiler/src/var_resolve/fresh_scalar_proposal.rs](../../../../rust/tcl-compiler/src/var_resolve/fresh_scalar_proposal.rs), `var_resolve::fresh_scalar_proposal::tests::original_fresh_scalar_proposal_requires_unoccupied_actual_frame_and_quiet_observers` (linked): Admits an independently entered empty local and declines occupied, aliased, observed, unknown incoming, qualified, array-shaped and clipped names.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the exact linked Rust selectors through the maintained workspace test setup. This record contains no interpreter observation or executed Rust result.
