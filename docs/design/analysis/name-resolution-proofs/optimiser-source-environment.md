# naming.variable.optimiser-source-environment

Kind: `implementation-contract`

## Problem statement

SSA display labels can name different cells after source aliases are rebound. Projecting constant lattice entries under those labels can associate a source condition with another cell or overwrite a differing constant.

## Question

Which source-name correspondence may project per-cell SCCP constants into the structure-elimination expression environment?

## Conclusion

Structure elimination projects SCCP constants only through the shared unpositioned SSA source view. Each admitted source spelling must select one proved symbol throughout the function, and every tracked lattice version of that symbol must agree on the same constant. SSA display labels supply no expression binding. A rebound spelling with distinct cell identities withdraws even when either cell has a constant and its presentation label is unchanged. This projection does not establish Normal reads, observer closure, rewrite equivalence or current variable contents beyond the existing lattice and source-view premises.

## Scope

Rust structure-elimination environment projection. The fixed source control retains genuine alias-read cell identities and versions, then supplies isolated constant lattice entries to test the projection boundary. It does not capture native outcomes or establish physical frame, alias lifetime or optimiser rewrite permission. No execution receipt for the selectors is attached.

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

- [rust/tcl-compiler/src/optimiser/structure_elimination.rs](../../../../rust/tcl-compiler/src/optimiser/structure_elimination.rs), `sccp_env_for`: Project unanimous source bindings and constant lattice versions without promoting display labels to expression identities.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `SsaSourceView`: Supply existing whole-function unanimous source bindings independently of SSA presentation labels.
- [rust/tcl-compiler/src/optimiser/structure_elimination.rs](../../../../rust/tcl-compiler/src/optimiser/structure_elimination.rs), `optimiser::structure_elimination::tests::original_sccp_environment_withdraws_rebound_source_names_despite_display_collisions` (linked): Retains two distinct genuine alias-read cell/version receipts and a reused display label; differing constant lattice entries cannot reintroduce the non-unanimous source spelling.

A named test is a coverage binding, not a claim that it executed.

## Replay

Fixed Rust selectors are linked without an attached execution receipt. Genuine source cell correspondence and isolated lattice constants do not grant Normal completion, observer closure, physical Native frames or source rewrite equivalence.
