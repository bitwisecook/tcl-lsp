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

- `selected-descriptor` (implementation): [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs). SHA-256 `f44e76a08cea2d79adb3051499b0410ea0fc7bc355e276c050345c413900890a`. Registry-selected descriptor and regression assertion source; no test execution outcome is attached.
- `effect-merge` (implementation): [rust/tcl-registry/src/world_effect.rs](../../../../rust/tcl-registry/src/world_effect.rs). SHA-256 `8e200733bc23f4e06b1d6831995fe6869c878bbb16453418603229eedc3a3e5e`. Shared source-qualified transition coverage and callback composition owner.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `PACKAGE_REGISTRATION_EFFECTS`: Selected current handler effects; no Normal or native body grant.
- [rust/tcl-registry/src/world_effect.rs](../../../../rust/tcl-registry/src/world_effect.rs), `EffectFootprint::from_legacy_with_transition_coverage`: Removes only the covered write source and retains reads and callbacks.
- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `commands::tcl::package_::tests::package_ifneeded_stores_future_script_without_current_callback_effect` (linked): Checks every selected C release, current package-state effects, absent current script/unknown callbacks, and the retained require barrier.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the named Rust selectors in the coherent workspace before claiming a passing result. No native replay is attached to this implementation contract.
