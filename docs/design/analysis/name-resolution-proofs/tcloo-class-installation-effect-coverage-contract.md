# naming.tcloo.class-installation-effect-coverage-contract

Kind: `implementation-contract`

## Problem statement

A named class factory emits an explicit OnOkOnly command definition and also inherits a coarse command-table effect. Counting that installation twice can discard the source publication and prevent authentic factory dependency lookup. Removing all factory effects instead would suppress definition scripts and command traces that may change the world. Coverage must identify only the legacy installation write already owned by the transition.

## Question

Does the selected class creation transition cover its own legacy command-binding installation write while retaining script callbacks, command trace callbacks and dynamic target uncertainty?

## Conclusion

The selected class creation descriptor covers only the LegacyCommandTable CommandBindings write owned by the emitted transition. Definition SCRIPT and command TRACE callbacks remain observable, and an unknown named target still widens CommandBindings. This coverage contract does not prove body completion, trace absence, entered factory behavior or native allocation.

## Scope

Rust Registry effect/transition composition for selected TclOO class creation on Tcl 8.6, 9.0 and 9.1. The fixed body could mutate the factory if entered, and a separate dynamic target retains widening. The test does not execute those bodies or establish native callback effects on any provider.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust Registry effect/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust Registry effect/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust Registry effect/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust Registry effect/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust Registry effect/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This is a Rust Registry effect/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This is a Rust Registry effect/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Exact evidence

- `selected-descriptor` (implementation): [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs). SHA-256 `1e5385471952e2572deb3898b347197b4cb54cfc2436425d7aa0891305a9f76f`. Registry-selected descriptor and regression assertion source; no test execution outcome is attached.
- `effect-merge` (implementation): [rust/tcl-registry/src/world_effect.rs](../../../../rust/tcl-registry/src/world_effect.rs). SHA-256 `8e200733bc23f4e06b1d6831995fe6869c878bbb16453418603229eedc3a3e5e`. Shared source-qualified transition coverage and callback composition owner.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs), `CLASS_INTERP_EFFECT_COVERAGE`: Selected current handler effects; no Normal or native body grant.
- [rust/tcl-registry/src/world_effect.rs](../../../../rust/tcl-registry/src/world_effect.rs), `EffectFootprint::from_legacy_with_transition_coverage`: Removes only the covered write source and retains reads and callbacks.
- [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs), `commands::tcl::oo_class::tests::class_creation_owns_installation_write_but_keeps_body_and_trace_callbacks` (linked): Checks source-qualified coverage, absent duplicate command-binding writes, retained script/trace callback barriers, and unknown-target widening.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the named Rust selectors in the coherent workspace before claiming a passing result. No native replay is attached to this implementation contract.
