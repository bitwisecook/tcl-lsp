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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust Registry effect/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs), `CLASS_INTERP_EFFECT_COVERAGE`: Selected current handler effects; no Normal or native body grant.
- [rust/tcl-registry/src/world_effect.rs](../../../../rust/tcl-registry/src/world_effect.rs), `EffectFootprint::from_legacy_with_transition_coverage`: Removes only the covered write source and retains reads and callbacks.
- [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs), `commands::tcl::oo_class::tests::class_creation_owns_installation_write_but_keeps_body_and_trace_callbacks` (linked): Checks source-qualified coverage, absent duplicate command-binding writes, retained script/trace callback barriers, and unknown-target widening.

## Evidence

- [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs)
- [rust/tcl-registry/src/world_effect.rs](../../../../rust/tcl-registry/src/world_effect.rs)
