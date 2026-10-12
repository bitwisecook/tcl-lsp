# naming.alias.original-fresh-creation-transfer

Kind: `implementation-contract`

## Problem statement

An interpreter alias retains a command prefix without executing its future target. Rejecting every missing future target blocks valid source alias installation, while treating a publication as normal completion can admit a loop, replacement cleanup or an observer. The source continuation therefore needs independent fresh-slot, captured-argv and alias-chain obligations.

## Question

Does a quiet same-interpreter C alias installation preserve a complete modeled normal world only after its fresh current command allocation and full captured target recipe are independently validated?

## Conclusion

The intrinsic receipt requires the current selected stock alias handler, exact original inputs, empty source and target interpreter paths, an existing vacant destination and a closed alias-chain check. A missing future target ends that check legally. The receipt checks the actual installed token, source allocation and captured target recipe before preserving normal completion. Occupied slots, loops, unknown paths, foreign interpreters and observed operations remain outside this envelope; no future dispatch or native preparation is granted.

## Scope

Implementation contract for source-only C Tcl 8.4 through 9.1 analysis and genuine frozen whole-argv inputs. The shared cycle kernel consumes caller-owned identities and preserves lookup errors. Native interpreter entry snapshots, replacement cleanup, imported alias wrappers, unknown callbacks, Jim alias cycle policy and target execution are independent obligations. No native provider result is asserted.

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

- [rust/tcl-syntax/src/naming/alias_chain.rs](../../../../rust/tcl-syntax/src/naming/alias_chain.rs), `alias_chain_loops`: Common topology traversal over independently selected actual alias identities; no lookup or interpreter lifetime grant.
- [rust/tcl-compiler/src/command_binding/original_alias_creation.rs](../../../../rust/tcl-compiler/src/command_binding/original_alias_creation.rs), `OriginalAliasCreation`: Independent selected-handler, fresh-cell and pre/post allocation premises for source alias normal completion.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `OriginalAliasTarget`: Retain authentic target and captured prefix producers through the same canonical command operation.
- [rust/tcl-registry/src/commands/tcl/interp.rs](../../../../rust/tcl-registry/src/commands/tcl/interp.rs), `spec`: Keep Registry alias interpreter-policy reads independent of precise binding lifecycle transitions and possible trace callbacks; current source transfer owns its separate Normal certificate.
- [rust/tcl-compiler/src/command_binding/original_alias_creation.rs](../../../../rust/tcl-compiler/src/command_binding/original_alias_creation.rs), `command_binding::original_alias_creation::tests::original_alias_creation_closes_only_fresh_quiet_actual_transfers` (linked): Fresh missing-target, builtin-prefix, source-procedure-prefix and opaque-name aliases retain a complete normal world; loops, occupied slots, nonempty interpreter paths and replacement do not borrow this envelope.
- [rust/tcl-syntax/src/naming/alias_chain.rs](../../../../rust/tcl-syntax/src/naming/alias_chain.rs), `naming::alias_chain::tests::original_alias_chain_keeps_missing_unknown_and_cycles_separate` (linked): A finite chain terminates, a repeated start or intermediate node is a loop, and unknown lookup remains an error.
- [rust/tcl-registry/src/commands/tcl/interp.rs](../../../../rust/tcl-registry/src/commands/tcl/interp.rs), `commands::tcl::interp::tests::original_alias_effects_separate_policy_reads_from_binding_lifecycle` (linked): Selected C Registry alias query/create/delete metadata retains interpreter-policy reads independently of binding lifecycle mutations and trace callbacks; unrelated interpreter creation retains its own write transition.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact Rust selectors listed above exercise the implementation premises. No execution receipt or native provider observation is attached to this implementation contract.
