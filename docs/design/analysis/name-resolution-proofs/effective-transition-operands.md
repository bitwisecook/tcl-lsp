# naming.invocation.effective-transition-operands

Kind: `implementation-contract`

## Problem statement

Two effective operands can have equal literal values while originating from different written words or captured prefix positions. A private ensemble worker also removes prepended subcommand words. Dropping a known operand ordinal or shifting only the top-level argument offset can select the wrong producer for a state mutation.

## Question

Do known and unknown transition subjects preserve their effective post-head ordinals through complete nested argv projection, while explicit authored values remain without source provenance?

## Conclusion

LocatedLiteral and Unknown retain exact effective ordinals; Literal remains explicitly authored. The central projection visits every nested subject and preserves literal facets, unknown kinds, fact order and completion commits. Any unmappable ordinal withdraws the whole projection. A transformed value facet still requires its independently selected Registry purpose.

## Scope

Rust implementation contract under the explicit contexts used by the linked tests. This record makes no C Tcl, Jim or BIG-IP observation claim, no Native entry claim and no successful evaluation claim.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `project_argument_indices`: Own complete nested effective argv projection without equality heuristics.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `argument_index`: Expose source ordinal only for independently located invocation operands.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::known_equal_operands_retain_distinct_effective_ordinals` (linked): Equal known values at distinct ordinals remain distinct; authored values do not acquire an ordinal.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::argv_projection_retains_nested_operands_order_and_completion_commits` (linked): Project nested alias/frame subjects in order, retain abrupt-edge applicability and reject underflow.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `registry::tests::private_registration_facts_keep_actual_operand_indices` (linked): A selected private namespace path worker keeps argument zero for both known and dynamic operands.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "--manifest-path",
  "rust/Cargo.toml",
  "state_transition::tests::known_equal_operands_retain_distinct_effective_ordinals"
]
```

Run each listed selector in its owning crate; the argument vector shows the first selector. A coverage binding is not a recorded test execution. Native interpreter measurements are separate records.
