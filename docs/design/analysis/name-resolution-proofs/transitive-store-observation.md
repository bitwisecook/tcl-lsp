# naming.variable.transitive-store-observation

Kind: `implementation-contract`

## Problem statement

An assignment can remain observable through a remote cell or trace after all of its local value consumers disappear. Protecting only the initial dead-store pass lets the transitive fixpoint delete an externally visible store.

## Question

Does transitive dead-code elimination retain the same actual target observations that prevent direct store deletion?

## Conclusion

The transitive fixpoint retains every actual definition selected by the existing dead-store target-observation predicates. Those predicates inspect typed point-owned writes, callback and unknown-access observations, alias or remote namespace targets and the separate external-observation suppression policies. Removing a private consumer does not withdraw the original target obligation. Unbounded observations use the same withdrawal boundary in the direct and transitive passes. Def-use keys remain typed cells and versions; display labels supply only existing conservative suppression. This observation closure establishes no value, Normal completion, native frame, lifetime, source-motion or edit authority.

## Scope

Rust optimiser contract for retained source SSA definitions. The linked six-profile model control requires an actual remote namespace target and a removable private consumer. Existing RHS purity, read definedness and def-use prerequisites remain independent; no native interpreter outcome is attached.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/optimiser/elimination.rs](../../../../rust/tcl-compiler/src/optimiser/elimination.rs), `observed_store_definitions`: Carry the existing direct-store target observations into typed keep-forever definition keys before the transitive fixpoint.
- [rust/tcl-compiler/src/optimiser/elimination.rs](../../../../rust/tcl-compiler/src/optimiser/elimination.rs), `emit_adce`: Use the same unbounded observation boundary and retain observed target definitions separately from RHS and def-use proofs.
- [rust/tcl-compiler/src/optimiser/elimination.rs](../../../../rust/tcl-compiler/src/optimiser/elimination.rs), `dead_store_writes_observed`: Own the shared typed callback, dynamic, unknown and unknown-access observations for store targets.
- [rust/tcl-compiler/src/optimiser/elimination.rs](../../../../rust/tcl-compiler/src/optimiser/elimination.rs), `dead_store_name_observed`: Keep alias, remote namespace and external-observation suppression separate from actual SSA cell identity.
- [rust/tcl-compiler/src/optimiser/elimination.rs](../../../../rust/tcl-compiler/src/optimiser/elimination.rs), `optimiser::elimination::tests::original_adce_keeps_actual_remote_stores_after_local_consumers_disappear` (linked): Across six selected source dialects, the actual remote namespace store remains while a private unused consumer qualifies for deletion; the asserted target comes from the retained point-owned Place rather than its reporting label.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the exact linked Rust selectors through the maintained workspace test setup. No native provider observation or executed Rust result is attached to this implementation record.
