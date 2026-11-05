# naming.package.ifneeded-current-effect-contract

Kind: `implementation-contract`

## Problem statement

A package index may register two loaders in sequence. Treating the first stored loader script as a current arbitrary callback can withdraw command lookup for the second registration; treating all package operations as inert can miss the actual callbacks and source loading performed by require. The selected registration leaf must describe its current effects separately from future execution.

## Question

Does the Registry ifneeded leaf describe current package registration without a current script callback while the separately selected require leaf retains an open callback effect?

## Conclusion

The selected ifneeded descriptor retains current PackageState read/write effects and the deferred script operand without a current script or unknown callback effect. The require leaf retains its independent open callback barrier. This effect contract supplies no successful completion, actual package availability, loader execution, body binding or native compiler grant.

## Scope

Rust Registry effect selection for the Tcl 8.4–9.1 ifneeded query and setter forms, contrasted with require. The fixed inputs include a stored script that would rename package and raise an error if evaluated. These are implementation assertions; no C Tcl, Jim or BIG-IP execution outcome is claimed.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust Registry effect/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `PACKAGE_REGISTRATION_EFFECTS`: Selected current handler effects; no Normal or native body grant.
- [rust/tcl-registry/src/world_effect.rs](../../../../rust/tcl-registry/src/world_effect.rs), `EffectFootprint::from_legacy_with_transition_coverage`: Removes only the covered write source and retains reads and callbacks.
- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `commands::tcl::package_::tests::package_ifneeded_stores_future_script_without_current_callback_effect` (linked): Checks every selected C release, current package-state effects, absent current script/unknown callbacks, and the retained require barrier.

## Evidence

- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs)
- [rust/tcl-registry/src/world_effect.rs](../../../../rust/tcl-registry/src/world_effect.rs)
