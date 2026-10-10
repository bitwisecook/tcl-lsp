# naming.command.original-occupied-mutation-transfer

Kind: `implementation-contract`

## Problem statement

A command lookup path can contain an earlier missing cell followed by the actual occupied cell. Treating the compiler traversal as a list of mutation targets can reject a unique rename or remove the wrong cell. A successful source publication transfer also needs independent handler, observer and allocation obligations before it can preserve a complete normal world.

## Question

Does the canonical source command mutation owner select the unanimous first occupied raw cell and preserve a complete normal move only after the same intrinsic target survives at an existing vacant destination?

## Conclusion

The occupied-cell selector declines missing, unknown, partially missing or disagreeing paths. The closed move receipt admits unobserved source-only C Tcl intrinsic procedure or builtin moves into an existing vacant namespace and checks the unchanged token and implementation allocation after the canonical transfer. An imported wrapper is admitted only when its retained origin token resolves to one direct intrinsic implementation and a unique current direct table cell. Completion requires the exact wrapper at the destination and the unchanged direct origin cell, token and implementation. Its independently retained compiler token is preserved without granting compiler preparation. The receipt grants no native execution, body entry or destruction proof.

## Scope

Implementation contract for retained original inputs and canonical modeled source command cells. The occupied selector is policy-selected; the complete normal move envelope is C Tcl 8.4 through 9.1 source-only analysis. The imported envelope retains the canonical C CommandToken wrapper and a direct procedure or builtin origin; source-name imports, imported interp aliases, ambiguous origins and unavailable object rows remain excluded. Native entry snapshots, command observers, interp aliases, object teardown, deletion, unknown targets, missing destination namespaces and unrepresented handler effects are excluded. No native provider result is asserted.

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

- [rust/tcl-compiler/src/command_binding/original_command_table.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_table.rs), `ModuleCommandBindings::original_occupied_command_for_input`: Canonical raw occupied-cell selection from genuine source inputs.
- [rust/tcl-compiler/src/command_binding/original_command_mutation.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_mutation.rs), `OriginalCommandMove`: Independent intrinsic move premises and post-transfer token/allocation checks.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `CapturedOriginalCommandTransfer`: Same-operation original source publication transfer; does not independently issue normal completion.
- [rust/tcl-compiler/src/command_binding/original_command_table.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_table.rs), `command_binding::original_command_table::tests::original_mutation_selector_stops_at_the_first_occupied_cell` (linked): Earlier missing cells do not become mutation targets; an earlier occupied, unknown or partly missing cell prevents fallback.
- [rust/tcl-compiler/src/command_binding/original_command_mutation.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_mutation.rs), `command_binding::original_command_mutation::tests::original_command_move_retains_the_allocation_in_a_complete_normal_world` (linked): A represented procedure move keeps its original declaration allocation in a complete modeled normal world.
- [rust/tcl-compiler/src/command_binding/original_command_mutation.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_mutation.rs), `command_binding::original_command_mutation::tests::original_command_move_does_not_close_missing_occupied_or_observed_targets` (linked): Missing sources, occupied destinations, observed moves and deletion do not borrow this move completion envelope.
- [rust/tcl-compiler/src/command_binding/original_command_mutation.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_mutation.rs), `command_binding::original_command_mutation::tests::original_imported_command_move_keeps_the_wrapper_and_direct_origin` (linked): A moved opaque imported command retains its exact wrapper, original definition and a complete modeled normal world under each C policy.
- [rust/tcl-compiler/src/command_binding/original_command_mutation.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_mutation.rs), `command_binding::original_command_mutation::tests::original_imported_command_move_keeps_origin_and_observer_barriers` (linked): Occupied destinations, rename observers and deletion do not borrow the imported move envelope.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact Rust selectors listed above exercise the implementation premises. No execution receipt or native provider observation is attached to this implementation contract.
