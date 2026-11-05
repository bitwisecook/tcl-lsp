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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

## Consumer bindings

- [rust/tcl-compiler/src/var_resolve/fresh_scalar_proposal.rs](../../../../rust/tcl-compiler/src/var_resolve/fresh_scalar_proposal.rs), `fresh_scalar_variable_slot`: Requires actual complete local and observer inventories under exact selected native name purpose.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `fresh_scalar_variable_proposal`: Binds the absence receipt to a genuine reached original source operation/configuration.
- [rust/tcl-compiler/src/var_resolve/fresh_scalar_proposal.rs](../../../../rust/tcl-compiler/src/var_resolve/fresh_scalar_proposal.rs), `var_resolve::fresh_scalar_proposal::tests::original_fresh_scalar_proposal_requires_unoccupied_actual_frame_and_quiet_observers` (linked): Admits an independently entered empty local and declines occupied, aliased, observed, unknown incoming, qualified, array-shaped and clipped names.
